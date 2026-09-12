"""Deterministic Ink journeys, using agent-tools build and device ownership."""

import json
import math
import re
import shutil
import subprocess
import sys
import time

from PIL import Image, ImageChops

APP = "benchmarks/apps/ink-stress"
PROTOCOL = 2


def percentile(values, fraction):
    values = sorted(values)
    return values[max(0, math.ceil(len(values) * fraction) - 1)] if values else None


def renderer_metrics(log):
    records = [
        {key: int(value) for key, value in re.findall(r"(\w+)=(\d+)", line)}
        for line in log.splitlines()
        if "Perf revision=" in line
    ]
    timing = {}
    for key in [
        "frame_ns",
        "relayout_ns",
        "prepare_ns",
        "upload_ns",
        "submit_present_ns",
    ]:
        values = [record[key] / 1_000_000 for record in records if key in record]
        timing[key.removesuffix("_ns") + "Ms"] = {
            "p95": percentile(values, 0.95),
            "maximum": max(values, default=None),
        }
    return {
        "frames": len(records),
        **timing,
        "uploadedBytes": sum(record.get("uploaded_bytes", 0) for record in records),
        "cacheMisses": sum(record.get("cache_misses", 0) for record in records),
    }


class Journey:
    def __init__(self, tools, device, artifact, directory, ocr):
        self.tools, self.device, self.artifact = tools, device, artifact
        self.directory, self.ocr = directory, ocr
        self.package = artifact["package"]
        self.pid = tools.cpu_ticks(device, self.package)[0]
        self.actions = []
        self.lines = []
        self.image = None

    def healthy(self):
        self.device.foreground(self.package)
        if self.tools.cpu_ticks(self.device, self.package)[0] != self.pid:
            raise RuntimeError("App restarted during the journey")

    def check_errors(self):
        errors = self.device.adb(
            "logcat",
            "-d",
            "--pid",
            self.pid,
            "-v",
            "brief",
            "-s",
            "Ink:E",
            "AndroidRuntime:E",
            "*:S",
        )
        if re.search(r"^[EF]/", errors, re.MULTILINE):
            (self.directory / "errors.txt").write_text(errors)
            raise RuntimeError("Unexpected runtime error; see errors.txt")

    def record(self, action):
        self.actions.append(action)
        with (self.directory / "actions.jsonl").open("a") as stream:
            stream.write(json.dumps(action) + "\n")

    def capture(self, path):
        self.healthy()
        self.device.screenshot(path)
        if sys.platform == "darwin":
            self.lines = json.loads(self.tools.run([*self.ocr, path], timeout=10))
        else:
            output = self.tools.run([*self.ocr, path, "stdout", "tsv"], timeout=10)
            # Group words into lines to use the same labels on both OCR providers.
            groups = {}
            for row in output.splitlines()[1:]:
                fields = row.split("\t", 11)
                if len(fields) != 12 or fields[0] != "5" or not fields[11].strip():
                    continue
                key = tuple(fields[1:5])
                left, top, width, height = map(int, fields[6:10])
                group = groups.setdefault(
                    key,
                    {
                        "words": [],
                        "left": left,
                        "top": top,
                        "right": left,
                        "bottom": top,
                    },
                )
                group["words"].append(fields[11])
                group["right"] = max(group["right"], left + width)
                group["bottom"] = max(group["bottom"], top + height)
            self.lines = [
                {
                    "text": " ".join(g["words"]),
                    "boundsNormalisedBottomLeft": [
                        g["left"] / 1080,
                        1 - g["bottom"] / 1240,
                        (g["right"] - g["left"]) / 1080,
                        (g["bottom"] - g["top"]) / 1240,
                    ],
                }
                for g in groups.values()
            ]
        for line in self.lines:
            line["text"] = line["text"].translate(str.maketrans("АВ", "AB"))
        self.tools.write_json(path.with_suffix(".json"), self.lines)
        with Image.open(path) as image:
            self.image = image.convert("RGB")

    def has_artwork(self):
        # The thumbnail column must contain substantial colour, not just text.
        red, green, blue = self.image.crop((0, 200, 280, 1080)).split()
        spread = ImageChops.subtract(
            ImageChops.lighter(red, ImageChops.lighter(green, blue)),
            ImageChops.darker(red, ImageChops.darker(green, blue)),
        )
        return sum(spread.histogram()[60:]) > 3000

    def expect(self, name, labels=(), artwork=False, scrolled=False):
        started = time.monotonic()
        path = self.directory / f"{len(self.actions):03}-{name}.png"
        previous = None
        while time.monotonic() - started < 10:
            self.capture(path)
            text = [line["text"] for line in self.lines]
            rows = [
                int(match[1])
                for line in text
                if (match := re.fullmatch(r"Album (\d{3})", line))
            ]
            if (
                all(label in text for label in labels)
                and (not artwork or self.has_artwork())
                and (
                    not scrolled
                    or (
                        rows
                        and min(rows) > 1
                        and previous is not None
                        and ImageChops.difference(previous, self.image).getbbox()
                        is None
                    )
                )
                and time.monotonic() - started < 10
            ):
                self.record(
                    {
                        "check": name,
                        "expected": list(labels),
                        "status": "passed",
                        "observedAfterMs": round((time.monotonic() - started) * 1000),
                        "screenshot": path.name,
                    }
                )
                return
            previous = self.image
            time.sleep(0.1)
        self.record(
            {
                "check": name,
                "expected": list(labels),
                "status": "failed",
                "observed": text,
                "screenshot": path.name,
            }
        )
        raise RuntimeError(
            f"Visible-state deadline exceeded: {name}; expected {labels}, saw {text}"
        )

    def tap(self, x, y):
        self.healthy()
        self.record({"input": "tap", "x": x, "y": y})
        self.device.shell("input", "tap", x, y)

    def tap_label(self, label):
        line = next(line for line in self.lines if line["text"] == label)
        x, y, width, height = line["boundsNormalisedBottomLeft"]
        self.tap(round((x + width / 2) * 1080), round((1 - y - height / 2) * 1240))

    def swipe(self, upwards, duration=350):
        self.healthy()
        start, end = (1000, 450) if upwards else (450, 1000)
        self.record(
            {
                "input": "swipe",
                "x": 540,
                "fromY": start,
                "toY": end,
                "durationMs": duration,
            }
        )
        self.device.shell("input", "swipe", 540, start, 540, end, duration)

    def back(self):
        self.record({"input": "back"})
        self.device.shell("input", "keyevent", "4")


def measure(tools, op, device, artifact, directory, args, ocr):
    launch = device.start(artifact["package"], artifact["component"])
    (directory / "launch.txt").write_text(launch)
    journey = Journey(tools, device, artifact, directory, ocr)
    package = artifact["package"]
    hz = int(device.shell("getconf", "CLK_TCK").strip())
    result = {"cycles": [], "completedCycles": 0}
    log_path = directory / "runtime.log"
    with log_path.open("wb") as log:
        collector = subprocess.Popen(
            [
                tools.ADB,
                "-s",
                device.serial,
                "logcat",
                "--pid",
                journey.pid,
                "-v",
                "threadtime",
                "-s",
                "Ink:V",
                "AndroidRuntime:E",
                "*:S",
            ],
            stdout=log,
            stderr=subprocess.STDOUT,
        )
        try:
            journey.expect(
                "launch", ["Library A", "Ready 1", "Album 001"], artwork=True
            )
            result["before"] = tools.memory_sample(device, package, directory, "before")
            for cycle in range(1, args["cycles"] + 1):
                op.step(f"{directory.name}: cycle {cycle}/{args['cycles']}")
                device.thermal()
                label = "A" if cycle % 2 else "B"
                revision = cycle * 2 - 1
                journey.tap_label("Album 001")
                journey.expect("detail", ["Album 001", "Album details"], artwork=True)
                journey.back()
                journey.expect(
                    "return", [f"Library {label}", "Album 001"], artwork=True
                )

                # No OCR or screenshots inside this measured continuous drag.
                device.shell("dumpsys", "SurfaceFlinger", "--timestats", "-clear")
                _, cpu_before = tools.cpu_ticks(device, package)
                started = time.monotonic()
                journey.swipe(True, 5000)
                time.sleep(0.25)
                _, cpu_after = tools.cpu_ticks(device, package)
                sample = {
                    "cycle": cycle,
                    "scrollCpuMs": (cpu_after - cpu_before) * 1000 / hz,
                    "scrollElapsedMs": (time.monotonic() - started) * 1000,
                    "scroll": tools.frames(
                        device, package, directory / f"cycle-{cycle}-frames.txt"
                    ),
                }
                journey.expect(
                    "scrolled", [f"Library {label}"], artwork=True, scrolled=True
                )
                for _ in range(6):
                    journey.swipe(True)
                journey.expect(
                    "deep-scroll", [f"Library {label}"], artwork=True, scrolled=True
                )
                selected = next(
                    line["text"]
                    for line in journey.lines
                    if re.fullmatch(r"Album \d{3}", line["text"])
                )
                journey.tap_label(selected)
                journey.expect(
                    "scrolled-detail", [selected, "Album details"], artwork=True
                )
                journey.back()
                journey.expect(
                    "scrolled-return", [f"Library {label}", selected], artwork=True
                )
                for _ in range(10):
                    journey.swipe(False)
                journey.expect("top", ["Album 001"], artwork=True)

                journey.tap(984, 64)
                journey.expect(
                    "refresh-error", [f"Retry {revision}", "Album 001"], artwork=True
                )
                journey.tap(984, 64)
                journey.expect(
                    "refresh-retry",
                    [f"Ready {revision + 2}", "Album 001"],
                    artwork=True,
                )
                journey.tap(962, 1150)
                journey.expect(
                    "settings", ["Settings", f"Dataset {label}", "Change dataset"]
                )
                journey.tap_label("Change dataset")
                next_label = "B" if label == "A" else "A"
                journey.expect("changed", [f"Dataset {next_label}"])
                journey.tap(118, 1150)
                journey.expect(
                    "changed-library",
                    [f"Library {next_label}", f"Ready {revision + 2}", "Album 001"],
                    artwork=True,
                )
                sample["memory"] = tools.memory_sample(
                    device, package, directory, f"cycle-{cycle}"
                )
                result["cycles"].append(sample)
                result["completedCycles"] = cycle
                tools.write_json(directory / "result.json", result)
                journey.check_errors()
            time.sleep(1)
            log_offset = log_path.stat().st_size
            _, cpu_before = tools.cpu_ticks(device, package)
            started = time.monotonic()
            time.sleep(args["idle_seconds"])
            _, cpu_after = tools.cpu_ticks(device, package)
            idle_elapsed = time.monotonic() - started
            idle_log = log_path.read_bytes()[log_offset:].decode(errors="replace")
            result["idle"] = {
                "elapsedSeconds": idle_elapsed,
                "cpuMs": (cpu_after - cpu_before) * 1000 / hz,
                "renderedFrames": renderer_metrics(idle_log)["frames"],
            }
            journey.healthy()
            journey.check_errors()
            if collector.poll() is not None:
                raise RuntimeError("Runtime log collection stopped unexpectedly")
            result["after"] = tools.memory_sample(device, package, directory, "after")
            result["memoryGrowthMiB"] = (
                result["after"]["pssKiB"] - result["before"]["pssKiB"]
            ) / 1024
            result["status"] = "passed"
        except BaseException as error:
            result["status"] = "failed"
            result["error"] = str(error) or type(error).__name__
            try:
                device.screenshot(directory / "failure.png")
                (directory / "failure-logcat.txt").write_text(
                    device.adb("logcat", "-d", "--pid", journey.pid, "-t", "2000")
                )
            except (RuntimeError, OSError, subprocess.SubprocessError) as capture_error:
                result["evidenceError"] = str(capture_error)
            raise
        finally:
            collector.terminate()
            try:
                collector.wait(timeout=5)
            except subprocess.TimeoutExpired:
                collector.kill()
                collector.wait()
            result["renderer"] = renderer_metrics(log_path.read_text(errors="replace"))
            if result.get("status") == "passed" and not result["renderer"]["frames"]:
                result.update(
                    status="failed",
                    error="Missing renderer metrics; build with INK_BENCHMARK=1",
                )
            tools.write_json(directory / "result.json", result)
    if result["status"] != "passed":
        raise RuntimeError(result["error"])
    return result


def stress(tools, op, args):
    builds = {"baseline": tools.load_build(args["baseline"], APP)}
    if args.get("candidate"):
        builds["candidate"] = tools.load_build(args["candidate"], APP)
    fixtures = []
    for path, manifest, artifact in builds.values():
        if artifact["package"] != "com.vandam.benchmark.ink.stress":
            raise ValueError("Stress runs require the Ink stress fixture package")
        if manifest["flags"].get("INK_BENCHMARK") != "1":
            raise ValueError("Stress runs require INK_BENCHMARK=1 builds")
        source = tools.read_json(path / "source.json")
        fixtures.append(
            {
                key: value
                for key, value in source["files"].items()
                if key.startswith(APP + "/")
            }
        )
    if len(builds) == 2 and (
        fixtures[0] != fixtures[1]
        or builds["baseline"][1]["flags"] != builds["candidate"][1]["flags"]
    ):
        raise ValueError(
            "Paired stress runs require identical fixture sources and instrumentation flags"
        )
    if sys.platform == "darwin":
        ocr = ["swift", tools.ROOT / "scripts/agent_tools_ocr.swift"]
    else:
        tesseract = shutil.which("tesseract")
        if not tesseract:
            raise RuntimeError("Stress checks require macOS Vision or Tesseract")
        ocr = [tesseract]
    shutil.copy2(__file__, op.path / "runner.py")
    shutil.copy2(tools.ROOT / "scripts/agent_tools.py", op.path / "agent_tools.py")
    shutil.copy2(tools.ROOT / "scripts/agent_tools_ocr.swift", op.path / "ocr.swift")
    device = tools.Device(args["serial"])
    session = device.acquire(op.state["id"])
    result = {
        "protocol": PROTOCOL,
        "device": session["device"],
        "serial": device.serial,
        "builds": {
            name: {"id": args[name], "manifest": build[1]}
            for name, build in builds.items()
        },
        "cyclesPerRun": args["cycles"],
        "idleSeconds": args["idle_seconds"],
        "rounds": args["rounds"],
        "samples": [],
        "notes": [
            "OCR deadlines verify visible states; observedAfterMs includes host/ADB/OCR overhead and is not input-to-display latency.",
            "SurfaceFlinger droppedFrames is not a complete count of missed display deadlines. Frame histograms have millisecond buckets.",
            "Renderer timings are instrumented wall times; submit_present includes waiting. PSS growth includes cache warm-up, not just leaks.",
            "Performance targets must be assessed on a physical LP3; emulator results are behaviour checks.",
        ],
    }
    try:
        if re.findall(r"\d+x\d+", device.shell("wm", "size"))[-1:] != ["1080x1240"]:
            raise RuntimeError("Stress coordinates require a 1080×1240 LP3 display")
        session["timestats"] = True
        device.save()
        device.shell("dumpsys", "SurfaceFlinger", "--timestats", "-enable")
        for round_number in range(1, args["rounds"] + 1):
            order = list(builds) if round_number % 2 else list(reversed(builds))
            for variant in order:
                path, _, artifact = builds[variant]
                directory = op.path / f"{round_number}-{variant}"
                directory.mkdir()
                op.step(f"{directory.name}: installing and launching")
                device.install(
                    artifact["package"], path / artifact["file"], artifact["sha256"]
                )
                sample = measure(tools, op, device, artifact, directory, args, ocr)
                result["samples"].append(
                    {"round": round_number, "variant": variant, **sample}
                )
                tools.write_json(op.path / "result.json", result)
                device.shell("am", "force-stop", artifact["package"])
        result["summary"] = {
            "status": "passed",
            "runs": len(result["samples"]),
            "completedCycles": sum(s["completedCycles"] for s in result["samples"]),
        }
    except BaseException as error:
        result["summary"] = {
            "status": "failed",
            "error": str(error) or type(error).__name__,
        }
        raise
    finally:
        try:
            tools.write_json(op.path / "result.json", result)
            report = [
                "# Ink stress test",
                "",
                f"Status: {result.get('summary', {}).get('status', 'incomplete')}",
                "",
                "| Run | Cycles | Scroll CPU median (ms) | Scroll interval p95 median (ms) | Prepare p95 (ms) | Upload (MiB) | PSS growth (MiB) | Idle frames |",
                "| --- | --- | --- | --- | --- | --- | --- | --- |",
            ]
            if result.get("summary", {}).get("error"):
                report[4:4] = [f"Failure: {result['summary']['error']}", ""]
            for sample in result["samples"]:
                cycles = sample["cycles"]
                report.append(
                    f"| {sample['round']}-{sample['variant']} | {sample['completedCycles']} | "
                    f"{tools.statistics.median(c['scrollCpuMs'] for c in cycles):.1f} | "
                    f"{tools.statistics.median(c['scroll']['presentP95Ms'] for c in cycles):.1f} | "
                    f"{sample['renderer']['prepareMs']['p95']:.2f} | {sample['renderer']['uploadedBytes'] / 1048576:.2f} | "
                    f"{sample['memoryGrowthMiB']:.2f} | {sample['idle']['renderedFrames']} |"
                )
            report += [
                "",
                *result["notes"],
                "",
                "Per-cycle samples, screenshots, OCR and logs are retained beside this report.",
            ]
            (op.path / "report.md").write_text("\n".join(report) + "\n")
        finally:
            device.release()
    return result
