"""Short renderer measurements against an already installed, foreground app."""

import re
import statistics
import time

from agent_stress import percentile, renderer_metrics


def bridge_metrics(log):
    samples = []
    scenes = {}
    presentations = {}
    current = None
    frame_started = None
    for line in log.splitlines():
        match = re.search(r"\b(InputUp|ReactDispatch|ReactDecode|ReactApply|ReactCommit|CommitRequested|FrameStart|Submitted|Presented|Presentation)\b (?=\w+=)(.*)", line)
        if not match:
            continue
        event, fields = match.groups()
        fields = {key: int(value) for key, value in re.findall(r"(\w+)=(\d+)", fields)}
        if event == "InputUp":
            current = {"input": fields["ns"], "dispatches": 0, "commits": 0}
            samples.append(current)
        elif event == "FrameStart":
            frame_started = fields["ns"]
        elif event == "Presentation":
            sample = presentations.get(fields["id"])
            if sample is not None:
                sample["displayed"] = fields["actual_ns"]
        elif event in ("Submitted", "Presented"):
            sample = scenes.get(fields["scene"])
            if sample is not None and "submitted" not in sample:
                sample["submitted"] = fields["ns"]
                if frame_started is not None and frame_started >= sample["commit"]:
                    sample["frameStarted"] = frame_started
                if "id" in fields:
                    presentations[fields["id"]] = sample
        elif current is not None:
            if event == "ReactDispatch":
                current["dispatches"] += 1
                current["dispatch"] = fields["ns"]
            elif event == "ReactDecode":
                current.update(decode=fields["ns"], decoded=fields["end_ns"], bytes=fields["bytes"])
            elif event == "ReactApply":
                current["apply"] = fields["ns"]
            elif event == "ReactCommit":
                current["commits"] += 1
                current["commit"] = fields["ns"]
                current["scene"] = fields["scene"]
                scenes[fields["scene"]] = current
            elif event == "CommitRequested" and "commit" in current:
                current.setdefault("requested", fields["ns"])
                current.setdefault("frameAlreadyPosted", fields["frame_posted"])

    stages = {
        "inputToDispatch": ("input", "dispatch"),
        "reactAndTransport": ("dispatch", "decode"),
        "jsonDecode": ("decode", "decoded"),
        "decodeToApply": ("decoded", "apply"),
        "applyAndLayout": ("apply", "commit"),
        "commitToFrameSubmission": ("commit", "submitted"),
        "inputToFrameSubmission": ("input", "submitted"),
        "commitToRequest": ("commit", "requested"),
        "requestToFrameStart": ("requested", "frameStarted"),
        "frameStartToSubmission": ("frameStarted", "submitted"),
        "inputToDriverPresentation": ("input", "displayed"),
    }
    accepted = []
    order = ["input", "dispatch", "decode", "decoded", "apply", "commit", "submitted"]
    for sample in samples:
        if sample["dispatches"] != 1 or sample["commits"] != 1:
            continue
        if not all(key in sample for key in order):
            continue
        timestamps = [sample[key] for key in order]
        if timestamps != sorted(timestamps):
            continue
        accepted.append(sample)
    if len(accepted) < 5:
        raise RuntimeError(
            "Insufficient isolated counter updates; use an INK_BRIDGE_TIMING=1 or INK_PRESENTATION_TIMING=1 "
            "build with ReactDecode instrumentation and inspect renderer.log"
        )
    timing = {}
    for name, (start, end) in stages.items():
        values = [
            (sample[end] - sample[start]) / 1_000_000 for sample in accepted
            if start in sample and end in sample and sample[end] >= sample[start]
        ]
        if not values:
            continue
        timing[name] = {
            "samples": len(values),
            "median": statistics.median(values),
            "p95": percentile(values, .95),
            "maximum": max(values),
        }
    return {
        "updates": len(accepted),
        "rejected": len(samples) - len(accepted),
        "milliseconds": timing,
        "medianCommitBytes": statistics.median(sample["bytes"] for sample in accepted),
        "samples": samples,
        "note": "Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.",
    }


def paced_metrics(log):
    commits = {}
    submitted = {}
    for line in log.splitlines():
        match = re.search(r"\b(ReactCommit|Submitted|Presented)\b (.*)", line)
        if not match:
            continue
        event, detail = match.groups()
        fields = {key: int(value) for key, value in re.findall(r"(\w+)=(\d+)", detail)}
        scene, timestamp = fields["scene"], fields["ns"]
        if event == "ReactCommit":
            commits[scene] = timestamp
        elif scene in commits and timestamp >= commits[scene]:
            submitted.setdefault(scene, timestamp)
    if len(submitted) < 5:
        raise RuntimeError("Insufficient paced updates; start ink-paced at rest with INK_BRIDGE_TIMING=1")
    times = sorted(submitted.values())
    intervals = [(end - start) / 1e6 for start, end in zip(times, times[1:])]
    return {
        "expectedUpdates": 180,
        "commits": len(commits),
        "submittedCommits": len(submitted),
        "unsubmittedCommits": len(commits) - len(submitted),
        "submissionsPerSecond": (len(times) - 1) * 1e9 / (times[-1] - times[0]),
        "submissionIntervalMs": {
            "median": statistics.median(intervals),
            "p95": percentile(intervals, .95),
            "maximum": max(intervals),
        },
        "note": "Software submission throughput for 180 scheduled 500-cell-and-resize updates. This is not per-update latency or physical display cadence. Fewer than 180 commits can indicate unfinished or batched updates.",
    }


def probe(tools, op, args):
    device = tools.Device(args["serial"])
    device.attach(args["token"])
    package = args["package"]
    device.foreground(package)
    device.thermal()
    if re.findall(r"\d+x\d+", device.shell("wm", "size"))[-1:] != ["1080x1240"]:
        raise RuntimeError("The probe requires a 1080×1240 display")
    hz = int(device.shell("getconf", "CLK_TCK").strip())
    marker = op.state["id"]
    device.shell("log", "-t", "InkProbe", marker)
    pid, before = tools.cpu_ticks(device, package)
    started = time.monotonic()
    if args["scenario"] == "paced":
        actions = 1
        device.shell("input", "tap", 984, 64)
        time.sleep(args["seconds"] + .2)
    elif args["scenario"] in ("counter", "bridge", "updates"):
        actions = args["seconds"] * 5
        for index in range(actions):
            x, y = (984, 64) if args["scenario"] == "updates" else (540, 760)
            device.shell("input", "tap", x, y)
            # Vary bridge taps across display phases instead of locking to 60 Hz.
            phase = [0, .007, .019, .031, .043][(index + 1) % 5] if args["scenario"] in ("bridge", "updates") else 0
            time.sleep(max(0, started + (index + 1) / 5 + phase - time.monotonic()))
    else:
        actions = 2
        duration = args["seconds"] * 500
        x = 994 if args["scenario"] == "scrollbar" else 540
        positions = [(560, 1050), (1050, 560)] if args["scenario"] == "scrollbar" else [(1050, 250), (250, 1050)]
        for first, last in positions:
            device.shell("input", "swipe", x, first, x, last, duration)
    elapsed = time.monotonic() - started
    if args["scenario"] in ("bridge", "updates"):
        time.sleep(.2)
    after_pid, after = tools.cpu_ticks(device, package)
    device.foreground(package)
    device.thermal()
    if pid != after_pid:
        raise RuntimeError("App restarted during the probe")
    log = device.adb(
        "logcat", "-d", "-v", "brief", "-s", "Ink", "InkProbe", "AndroidRuntime", "*:S"
    )
    if marker not in log:
        raise RuntimeError("Probe log marker was lost")
    log = log.split(marker, 1)[1]
    log = "\n".join(
        line for line in log.splitlines() if re.search(rf"\(\s*{pid}\)", line)
    )
    (op.path / "renderer.log").write_text(log + "\n")
    if re.search(r"^[EF]/", log, re.MULTILINE):
        raise RuntimeError("Runtime error during probe; see renderer.log")
    metrics = renderer_metrics(log)
    if metrics["frames"] < 5 and args["scenario"] not in ("bridge", "updates", "paced"):
        raise RuntimeError(
            "Insufficient frame evidence; use an INK_BENCHMARK=1 build on the matching fixture"
        )
    device.screenshot(op.path / "after.png")
    result = {
        "package": package,
        "pid": pid,
        "scenario": args["scenario"],
        "actions": actions,
        "seconds": round(elapsed, 3),
        "cpuMs": (after - before) * 1000 / hz,
        "renderer": metrics if metrics["frames"] >= 5 else None,
        "note": "Short iteration probe, not a paired benchmark or full compatibility check. Start each comparison at the same scroll position with the same cache state.",
    }
    if args["scenario"] in ("bridge", "updates"):
        bridge = bridge_metrics(log)
        result["bridge"] = bridge
        rows = [
            "# React to native update",
            "",
            f"Accepted updates: {bridge['updates']}; rejected: {bridge['rejected']}.",
            f"Median commit payload: {bridge['medianCommitBytes']:g} bytes.",
            "",
            "| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |",
            "| --- | ---: | ---: | ---: | ---: |",
        ]
        for stage, values in bridge["milliseconds"].items():
            label = re.sub(r"([A-Z])", r" \1", stage).capitalize()
            rows.append(
                f"| {label} | {values['samples']} | {values['median']:.3f} | {values['p95']:.3f} | {values['maximum']:.3f} |"
            )
        rows.extend(["", bridge["note"], "", "Raw timestamps and samples: [result.json](result.json).", ""])
        (op.path / "report.md").write_text("\n".join(rows))
    if args["scenario"] == "paced":
        result["paced"] = paced_metrics(log)
    tools.write_json(op.path / "result.json", result)
    return result
