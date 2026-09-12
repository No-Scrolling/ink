# /// script
# requires-python = ">=3.12"
# dependencies = ["pillow>=11,<13"]
# ///
"""Local, evidence-producing tools for Ink experiments. See docs/agent-tools.md."""

import argparse
import hashlib
import json
import math
import os
import re
import shlex
import shutil
import signal
import statistics
import subprocess
import sys
import tarfile
import time
import tomllib
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STORE = Path(os.environ.get("INK_AGENT_TOOLS_DIR", ROOT / ".agent-tools")).resolve()
LEASES = (
    Path(os.environ.get("XDG_STATE_HOME", Path.home() / ".local/state"))
    / "ink-agent-tools/devices"
)
ADB = os.environ.get(
    "ADB",
    shutil.which("adb") or str(Path.home() / "Library/Android/sdk/platform-tools/adb"),
)
SETTINGS = {
    "stay_on_while_plugged_in": "7",
    "window_animation_scale": "0",
    "transition_animation_scale": "0",
    "animator_duration_scale": "0",
}


def run(args, *, cwd=None, env=None, timeout=120, log=None, check=True):
    result = subprocess.run(
        [str(a) for a in args],
        cwd=cwd,
        env=env,
        timeout=timeout,
        stdout=log or subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    output = (
        result.stdout.decode(errors="replace").replace("\r", "")
        if result.stdout
        else ""
    )
    if check and result.returncode:
        raise RuntimeError(
            f"{Path(str(args[0])).name} exited {result.returncode}: {output[-3000:]}"
        )
    return output


def write_json(path, value):
    path = Path(path)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n")
    temporary.replace(path)


def read_json(path):
    return json.loads(Path(path).read_text())


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def emit(value):
    print(json.dumps(value), flush=True)


def operation_path(identifier):
    if not re.fullmatch(r"[a-z0-9-]+", identifier):
        raise ValueError("Use an operation ID returned by this tool")
    return STORE / identifier


def result_summary(kind, result):
    if kind in {"bench", "stress"}:
        return result.get(
            "summary", {"acceptedSamples": len(result.get("samples", []))}
        )
    if kind == "experiment":
        return {"apps": list(result["artifacts"]), "flags": result["flags"]}
    if kind == "image":
        summary = {
            key: result[key]
            for key in ["size", "regionPixels", "brightPixels", "colouredPixels"]
        }
        if "comparison" in result:
            summary["comparison"] = {
                key: result["comparison"][key]
                for key in ["changedPixels", "changedPercent", "changedBoundsInCrop"]
            }
        if "text" in result:
            text = result["text"]
            lines = (
                [line["text"] for line in text]
                if isinstance(text, list)
                else text.splitlines()
            )
            summary["text"] = [line[:200] for line in lines[:10]]
            if len(lines) > 10 or any(len(line) > 200 for line in lines[:10]):
                summary["textTruncated"] = True
        return summary
    if kind == "memory":
        android = result["android"]
        ink = result["ink"]
        return {
            "pssMiB": round(android["pssKiB"] / 1024, 2),
            "categoriesPssKiB": android["categoriesPssKiB"],
            "ink": {key: ink[key] for key in ["native", "renderer"]} if ink else None,
            "notes": result["notes"],
        }
    return result


def operation_view(state, full=False):
    if full:
        return state
    view = {
        "id": state["id"],
        "status": state["status"],
        "cursor": str(state.get("updated", state["created"])),
    }
    if state["status"] in {"queued", "running"} and "progress" in state:
        view["progress"] = state["progress"]
    if "error" in state:
        view["error"] = state["error"]
    path = operation_path(state["id"])
    if state.get("result"):
        result_path = path / state["result"]
        view["summary"] = result_summary(state["kind"], read_json(result_path))
        view["evidence"] = str(path)
    elif state["status"] not in {"queued", "running"}:
        view["evidence"] = str(path)
    return view


def wait_operation(identifier, after, timeout):
    deadline = time.monotonic() + timeout
    while True:
        state = read_json(operation_path(identifier) / "status.json")
        cursor = str(state.get("updated", state["created"]))
        finished = state["status"] not in {"queued", "running"}
        if (after is not None and cursor != after) or (after is None and finished):
            return operation_view(state)
        if finished or time.monotonic() >= deadline:
            return (
                {
                    "id": identifier,
                    "status": state["status"],
                    "cursor": cursor,
                    "unchanged": True,
                }
                if after == cursor
                else operation_view(state)
            )
        time.sleep(min(0.2, max(0, deadline - time.monotonic())))


class Operation:
    def __init__(self, path):
        self.path = Path(path)
        self.state = read_json(self.path / "status.json")

    def update(self, **changes):
        self.state.update(changes, updated=time.time())
        write_json(self.path / "status.json", self.state)

    def step(self, message):
        self.update(progress=message)
        with (self.path / "events.jsonl").open("a") as stream:
            stream.write(json.dumps({"time": time.time(), "message": message}) + "\n")
        emit({"id": self.state["id"], "progress": message})


class Device:
    def __init__(self, serial):
        self.serial = serial
        self.path = LEASES / hashlib.sha256(serial.encode()).hexdigest()
        self.session = None

    def adb(self, *args, **kwargs):
        return run([ADB, "-s", self.serial, *args], **kwargs)

    def shell(self, *args, **kwargs):
        return self.adb("shell", " ".join(shlex.quote(str(a)) for a in args), **kwargs)

    def save(self):
        write_json(self.path / "session.json", self.session)

    def acquire(self, owner):
        LEASES.mkdir(parents=True, exist_ok=True)
        try:
            self.path.mkdir()
        except FileExistsError:
            raise RuntimeError(
                f"Device reserved; inspect with device status --serial {self.serial}"
            ) from None
        self.session = {
            "serial": self.serial,
            "token": uuid.uuid4().hex,
            "owner": owner,
            "created": time.time(),
            "settings": {},
            "packages": [],
            "timestats": False,
        }
        self.save()
        try:
            if self.adb("get-state").strip() != "device":
                raise RuntimeError("Device is not available")
            for setting, value in SETTINGS.items():
                previous = self.shell("settings", "get", "global", setting).strip()
                self.session["settings"][setting] = {
                    "previous": previous,
                    "applied": value,
                }
                self.save()
                self.shell("settings", "put", "global", setting, value)
            self.shell("input", "keyevent", "224")
            self.shell("wm", "dismiss-keyguard")
            windows = self.shell("dumpsys", "window")
            if "Luma Unlock Gate" in windows:
                self.shell("input", "swipe", "540", "1150", "540", "300", "300")
            self.session["device"] = {
                key: self.shell("getprop", key).strip()
                for key in [
                    "ro.product.model",
                    "ro.build.version.release",
                    "ro.build.fingerprint",
                ]
            }
            self.save()
            return self.session
        except BaseException:
            self.release()
            raise

    def attach(self, token):
        self.session = read_json(self.path / "session.json")
        if self.session["token"] != token:
            raise RuntimeError("Reservation token does not match")

    def release(self):
        errors = []
        for package in self.session["packages"][:]:
            try:
                if self.shell("pm", "path", package, check=False).strip():
                    self.adb("uninstall", package)
                self.session["packages"].remove(package)
                self.save()
            except (RuntimeError, OSError, subprocess.SubprocessError) as error:
                errors.append(str(error))
        if self.session["timestats"]:
            try:
                self.shell("dumpsys", "SurfaceFlinger", "--timestats", "-disable")
                self.session["timestats"] = False
                self.save()
            except (RuntimeError, OSError, subprocess.SubprocessError) as error:
                errors.append(str(error))
        for key, values in list(self.session["settings"].items()):
            try:
                current = self.shell("settings", "get", "global", key).strip()
                if current == values["applied"]:
                    if values["previous"] == "null":
                        self.shell("settings", "delete", "global", key)
                    else:
                        self.shell("settings", "put", "global", key, values["previous"])
                del self.session["settings"][key]
                self.save()
            except (RuntimeError, OSError, subprocess.SubprocessError) as error:
                errors.append(str(error))
        if errors:
            raise RuntimeError(
                "Cleanup incomplete; reservation retained for device release: "
                + "; ".join(errors)
            )
        shutil.rmtree(self.path)

    def install(self, package, apk, sha256):
        if digest(apk) != sha256:
            raise RuntimeError("APK hash changed since the experiment was built")
        if package not in self.session["packages"]:
            if self.shell("pm", "path", package, check=False).strip():
                raise RuntimeError(
                    f"{package} is already installed; use a spare device or remove it yourself first"
                )
            self.session["packages"].append(package)
            self.save()
        self.adb("install", "-r", apk)

    def foreground(self, package):
        windows = self.shell("dumpsys", "window")
        focus = next(
            (line for line in windows.splitlines() if "mCurrentFocus=" in line), ""
        )
        if not re.search(r"\s" + re.escape(package) + r"/", focus):
            raise RuntimeError(
                f"Foreground interference: expected {package}, got {focus.strip()}"
            )
        if "Luma Unlock Gate" in focus:
            raise RuntimeError("Device is locked")
        return focus.strip()

    def thermal(self):
        dump = self.shell("dumpsys", "thermalservice")
        match = re.search(r"Thermal Status:\s+(\d+)", dump)
        if not match:
            raise RuntimeError("Android thermal status is unavailable")
        status = int(match[1])
        if status != 0:
            raise RuntimeError(
                f"Thermal interference: status {status}; sample rejected"
            )
        return status

    def start(self, package, component):
        for other in self.session["packages"]:
            self.shell("am", "force-stop", other)
        output = self.shell("am", "start", "-W", "-n", component)
        if "Status: ok" not in output:
            raise RuntimeError("Launch failed: " + output)
        time.sleep(2)
        self.foreground(package)
        self.thermal()
        return output

    def screenshot(self, path):
        with Path(path).open("wb") as stream:
            subprocess.run(
                [ADB, "-s", self.serial, "exec-out", "screencap", "-p"],
                stdout=stream,
                stderr=subprocess.PIPE,
                check=True,
                timeout=30,
            )


def memory_sample(device, package, directory, prefix):
    focus = device.foreground(package)
    device.thermal()
    dump = device.shell("dumpsys", "meminfo", package)
    (directory / f"{prefix}-meminfo.txt").write_text(dump)
    values = {"capturedAt": time.time()}
    for key, label in [
        ("pssKiB", "TOTAL PSS"),
        ("rssKiB", "TOTAL RSS"),
        ("swapPssKiB", "TOTAL SWAP PSS"),
    ]:
        match = re.search(label + r":\s+(\d+)", dump)
        values[key] = int(match[1]) if match else None
    if values["pssKiB"] is None:
        raise RuntimeError("PSS is missing from meminfo")
    summary = dump.split("App Summary", 1)[-1].split("TOTAL PSS", 1)[0]
    header = next(
        (
            line
            for line in summary.splitlines()
            if "Pss(KB)" in line and "Rss(KB)" in line
        ),
        None,
    )
    values["categoriesPssKiB"] = None
    values["categoriesRssKiB"] = None
    if header:
        rss_column = header.index("Rss(KB)")
        values["categoriesPssKiB"] = {}
        values["categoriesRssKiB"] = {}
        for line in summary.splitlines():
            if ":" not in line:
                continue
            label, _ = line.split(":", 1)
            for key, value in [
                ("categoriesPssKiB", line[line.index(":") + 1 : rss_column]),
                ("categoriesRssKiB", line[rss_column:]),
            ]:
                if value.strip().isdigit():
                    values[key][label.strip()] = int(value.strip())
    values["foreground"] = focus
    device.foreground(package)
    return values


def source_snapshot(args, destination):
    if args.get("ref"):
        revision = run(
            ["git", "rev-parse", "--verify", args["ref"] + "^{commit}"], cwd=ROOT
        ).strip()
        archive = destination.parent / "source.tar"
        run(
            ["git", "archive", "--format=tar", f"--output={archive}", revision],
            cwd=ROOT,
        )
        with tarfile.open(archive) as stream:
            stream.extractall(destination, filter="data")
        archive.unlink()
    else:
        revision = run(["git", "rev-parse", "HEAD"], cwd=ROOT).strip()
        names = run(
            ["git", "ls-files", "-c", "-o", "--exclude-standard", "-z"], cwd=ROOT
        ).split("\0")
        for name in sorted(set(names) - {""}):
            source = ROOT / name
            if source.is_relative_to(STORE):
                continue
            if not source.exists() and not source.is_symlink():
                continue
            target = destination / name
            target.parent.mkdir(parents=True, exist_ok=True)
            if source.is_symlink():
                target.symlink_to(os.readlink(source))
            elif source.is_file():
                shutil.copy2(source, target)
    files = {}
    for path in sorted(destination.rglob("*")):
        if path.is_symlink():
            files[str(path.relative_to(destination))] = {"symlink": os.readlink(path)}
        elif path.is_file():
            files[str(path.relative_to(destination))] = {
                "sha256": digest(path),
                "mode": path.stat().st_mode & 0o777,
            }
    return {
        "revision": revision,
        "workingTree": not bool(args.get("ref")),
        "files": files,
        "sha256": hashlib.sha256(
            json.dumps(files, sort_keys=True).encode()
        ).hexdigest(),
    }


def build(op, args):
    workspace = op.path / "workspace"
    workspace.mkdir()
    op.step("Snapshotting source")
    source = source_snapshot(args, workspace)
    write_json(op.path / "source.json", source)
    archive = op.path / "source.tar.gz"
    with tarfile.open(archive, "w:gz") as stream:
        stream.add(workspace, arcname=".")
    archive.chmod(0o444)
    flags = {}
    for entry in args["env"]:
        key, value = entry.split("=", 1)
        if key not in {
            "INK_SPLIT_WEB",
            "INK_BENCHMARK",
            "INK_PRESENTATION_TIMING",
            "INK_MEMORY_DIAGNOSTICS",
        } or value not in {"0", "1"}:
            raise ValueError(
                "Build flags support INK_SPLIT_WEB, INK_BENCHMARK, INK_PRESENTATION_TIMING and INK_MEMORY_DIAGNOSTICS, each 0 or 1"
            )
        flags[key] = value
    environment = {k: v for k, v in os.environ.items() if not k.startswith("INK_")}
    environment.update(
        flags,
        INK_SDK_ROOT=str(workspace),
        INK_KEYSTORE_PASSWORD="android",
        INK_KEY_PASSWORD="android",
        INK_BENCHMARK_REVISION=source["sha256"],
    )
    environment["CARGO_TARGET_DIR"] = str(ROOT / "target")
    op.step("Installing locked JavaScript dependencies")
    with (op.path / "build.log").open("w") as log:
        run(
            ["bun", "install", "--frozen-lockfile"],
            cwd=workspace,
            env=environment,
            log=log,
            timeout=600,
        )
        artifacts = {}
        for app in args["app"]:
            project = (workspace / app).resolve()
            if (
                not project.is_relative_to(workspace)
                or not (project / "ink.toml").exists()
            ):
                raise ValueError("App must be a project inside the source snapshot")
            config = tomllib.loads((project / "ink.toml").read_text())
            signing = config.get("signing", {})
            keystore = (
                project / signing.get("keystore", ".agent-tools-debug.keystore")
            ).resolve()
            if not keystore.is_relative_to(workspace):
                raise ValueError(
                    "Experiment signing path must stay inside the snapshot"
                )
            if not signing:
                with (project / "ink.toml").open("a") as stream:
                    stream.write(
                        '\n[signing]\nkeystore = ".agent-tools-debug.keystore"\nkey_alias = "androiddebugkey"\n'
                    )
            keystore.parent.mkdir(parents=True, exist_ok=True)
            alias = signing.get("key_alias", "androiddebugkey")
            shared_key = (
                STORE
                / "cache"
                / (hashlib.sha256(alias.encode()).hexdigest() + ".keystore")
            )
            shared_key.parent.mkdir(parents=True, exist_ok=True)
            if not shared_key.exists():
                temporary_key = shared_key.with_name(uuid.uuid4().hex + ".keystore")
                run(
                    [
                        "keytool",
                        "-genkeypair",
                        "-keystore",
                        temporary_key,
                        "-alias",
                        alias,
                        "-storepass",
                        "android",
                        "-keypass",
                        "android",
                        "-dname",
                        "CN=Ink Agent Tools",
                        "-keyalg",
                        "RSA",
                        "-validity",
                        "3650",
                        "-noprompt",
                    ],
                    log=log,
                )
                try:
                    os.link(temporary_key, shared_key)
                except FileExistsError:
                    pass
                finally:
                    temporary_key.unlink()
            shutil.copyfile(shared_key, keystore)
            op.step(f"Building {app}")
            run(
                [workspace / "scripts/ink", "-C", project, "build"],
                cwd=workspace,
                env=environment,
                log=log,
                timeout=1800,
            )
            apks = list((project / "dist").glob("*.apk"))
            if len(apks) != 1:
                raise RuntimeError(f"Expected one APK in {project / 'dist'}")
            target = op.path / f"{len(artifacts)}-{apks[0].name}"
            shutil.copyfile(apks[0], target)
            target.chmod(0o444)
            artifacts[app] = {
                "file": target.name,
                "sha256": digest(target),
                "bytes": target.stat().st_size,
                "package": config["package"],
                "component": config["package"] + "/com.vandam.ink.MainActivity",
            }
    manifest = {
        "sourceSha256": source["sha256"],
        "sourceArchive": {"file": archive.name, "sha256": digest(archive)},
        "revision": source["revision"],
        "flags": flags,
        "signing": "generated Android development key",
        "artifacts": artifacts,
        "tools": {
            "bun": run(["bun", "--version"]).strip(),
            "rust": run(["rustc", "--version"]).strip(),
        },
        "compilerEnvironment": {
            k: environment[k]
            for k in ["RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"]
            if k in environment
        },
    }
    write_json(op.path / "manifest.json", manifest)
    return manifest


def load_build(identifier, app):
    path = operation_path(identifier)
    state = read_json(path / "status.json")
    if state["status"] != "complete" or state["kind"] != "experiment":
        raise ValueError("Baseline and candidate must be completed experiment IDs")
    manifest = read_json(path / "manifest.json")
    artifact = manifest["artifacts"][app]
    if digest(path / artifact["file"]) != artifact["sha256"]:
        raise RuntimeError("Experiment APK no longer matches its manifest")
    return path, manifest, artifact


def cpu_ticks(device, package):
    pid = device.shell("pidof", "-s", package).strip()
    if not pid.isdigit():
        raise RuntimeError("App process exited")
    stat = device.shell("cat", f"/proc/{pid}/stat").rsplit(")", 1)[1].split()
    return pid, int(stat[11]) + int(stat[12])


def frames(device, package, path):
    device.foreground(package)
    dump = device.shell("dumpsys", "SurfaceFlinger", "--timestats", "-dump")
    path.write_text(dump)
    blocks = [b for b in re.split(r"(?=displayRefreshRate =)", dump) if package in b]

    def count(block):
        match = re.search(r"totalFrames = (\d+)", block)
        return int(match[1]) if match else 0

    block = max(blocks, key=count, default="")
    if not count(block):
        raise RuntimeError("No app frames recorded; cannot report frame timing")
    histogram = re.search(r"present2present histogram is as below:\n([^\n]+)", block)
    buckets = sorted(
        (int(ms), int(n))
        for ms, n in re.findall(r"(\d+)ms=(\d+)", histogram[1] if histogram else "")
    )
    if not sum(n for _, n in buckets):
        raise RuntimeError(
            "No presentation histogram recorded; cannot report frame timing"
        )

    def percentile(fraction):
        total = sum(n for _, n in buckets)
        seen = 0
        for ms, n in buckets:
            seen += n
            if total and seen >= math.ceil(total * fraction):
                return ms
        return None

    dropped = re.search(r"droppedFrames = (\d+)", block)
    return {
        "frames": count(block),
        "droppedFrames": int(dropped[1]) if dropped else None,
        "presentP95Ms": percentile(0.95),
        "presentP99Ms": percentile(0.99),
    }


def workload(device, artifact, scenario, directory, prefix):
    package = artifact["package"]
    device.shell("dumpsys", "SurfaceFlinger", "--timestats", "-clear")
    hz = int(device.shell("getconf", "CLK_TCK").strip())
    pid, before = cpu_ticks(device, package)
    if scenario == "counter":
        for _ in range(100):
            device.foreground(package)
            device.shell("input", "tap", "540", "760")
    else:
        for first, last in [(1050, 180), (180, 1050)]:
            for _ in range(6):
                device.foreground(package)
                device.shell("input", "swipe", "540", first, "540", last, "350")
    time.sleep(0.75)
    after_pid, after = cpu_ticks(device, package)
    if pid != after_pid:
        raise RuntimeError("App restarted during workload; sample rejected")
    result = {
        "cpuMs": (after - before) * 1000 / hz,
        "frames": frames(device, package, directory / f"{prefix}-frames.txt"),
        "memory": memory_sample(device, package, directory, prefix + "-after"),
    }
    if scenario == "scroll":
        device.start(package, artifact["component"])
        device.shell("dumpsys", "SurfaceFlinger", "--timestats", "-clear")
        device.shell("input", "swipe", "540", "1050", "540", "180", "5000")
        result["continuous"] = frames(
            device, package, directory / f"{prefix}-continuous.txt"
        )
    return result


def compare_frameworks(op, args):
    config = read_json(args["comparison"])
    packages = {
        "ink": "com.vandam.benchmark.ink.counter",
        "expo": "com.vandam.benchmark.expo.counter",
        "light-sdk": "com.vandam.benchmark.lightsdk.counter",
    }
    artifacts = {}
    for stack, package in packages.items():
        source = Path(config[stack]).resolve()
        target = op.path / f"{stack}.apk"
        shutil.copy2(source, target)
        target.chmod(0o444)
        artifacts[stack] = {"package": package, "file": str(target), "sha256": digest(target)}
    write_json(op.path / "artifacts.json", artifacts)
    shutil.copy2(ROOT / "benchmarks/measure.ts", op.path / "measure.ts")
    device = Device(args["serial"])
    session = device.acquire(op.state["id"])
    write_json(op.path / "settings-before.json", session["settings"])
    try:
        if re.findall(r"\d+x\d+", device.shell("wm", "size"))[-1:] != ["1080x1240"]:
            raise RuntimeError("Counter comparison requires a 1080×1240 display")
        session["timestats"] = True
        device.save()
        for stack, artifact in artifacts.items():
            op.step(f"Installing {stack} counter")
            device.install(artifact["package"], artifact["file"], artifact["sha256"])
        env = {**os.environ, "BENCHMARK_DEVICE": device.serial,
               "BENCHMARK_STACKS": "ink,expo,light-sdk", "BENCHMARK_SCENARIOS": "Counter",
               "BENCHMARK_OUTPUT": str(op.path / "result.json"),
               "BENCHMARK_EVIDENCE_DIR": str(op.path),
               "INK_COUNTER_APK": artifacts["ink"]["file"],
               "EXPO_COUNTER_APK": artifacts["expo"]["file"],
               "LIGHT_COUNTER_APK": artifacts["light-sdk"]["file"]}
        op.step("Measuring three counters: 15 launches, five idle samples and five 100-tap workloads each")
        with (op.path / "runtime.log").open("w") as log:
            run(["bun", op.path / "measure.ts"], cwd=ROOT, env=env, timeout=1800, log=log)
        result = read_json(op.path / "result.json")
        if result["environment"]["thermalStatusEnd"] != 0:
            raise RuntimeError("Device heated during comparison; discard the run")
        result["summary"] = {"apps": len(result["results"]), "workloads": sum(len(r["workload"]["samples"]) for r in result["results"])}
        write_json(op.path / "result.json", result)
        return result
    finally:
        op.step("Removing benchmark apps and restoring device settings")
        device.release()
        write_json(op.path / "settings-after.json", {
            key: device.shell("settings", "get", "global", key).strip() for key in SETTINGS
        })
        (op.path / "packages-after.txt").write_text(device.shell("pm", "list", "packages", "com.vandam.benchmark"))


def compare(op, args):
    if args.get("comparison"):
        return compare_frameworks(op, args)
    if not all(args.get(key) for key in ["baseline", "candidate", "app"]):
        raise ValueError("Paired benchmarks require --baseline, --candidate and --app")
    builds = {
        variant: load_build(args[variant], args["app"])
        for variant in ["baseline", "candidate"]
    }
    if builds["baseline"][2]["package"] != builds["candidate"][2]["package"]:
        raise ValueError("Paired APKs must have the same package")
    device = Device(args["serial"])
    session = device.acquire(op.state["id"])
    result = {
        "builds": {v: {"id": args[v], "manifest": b[1]} for v, b in builds.items()},
        "device": session["device"],
        "serial": device.serial,
        "scenario": args["scenario"],
        "fixture": args["app"],
        "samples": [],
        "rounds": args["rounds"],
    }
    try:
        if args["scenario"] != "idle":
            size = device.shell("wm", "size")
            if re.findall(r"\d+x\d+", size)[-1:] != ["1080x1240"]:
                raise RuntimeError(
                    "Interaction presets require a 1080×1240 LP3 display; use --scenario idle elsewhere"
                )
            session["timestats"] = True
            device.save()
            (op.path / "timestats-before.txt").write_text(
                device.shell("dumpsys", "SurfaceFlinger", "--timestats", "-dump")
            )
            device.shell("dumpsys", "SurfaceFlinger", "--timestats", "-enable")
        for round_number in range(1, args["rounds"] + 1):
            order = (
                ["baseline", "candidate"]
                if round_number % 2
                else ["candidate", "baseline"]
            )
            for variant in order:
                path, _, artifact = builds[variant]
                prefix = f"{round_number}-{variant}"
                op.step(f"Round {round_number}/{args['rounds']}: {variant}")
                device.thermal()
                device.install(
                    artifact["package"], path / artifact["file"], artifact["sha256"]
                )
                launch = device.start(artifact["package"], artifact["component"])
                (op.path / f"{prefix}-launch.txt").write_text(launch)
                sample = {
                    "round": round_number,
                    "variant": variant,
                    "idle": memory_sample(
                        device, artifact["package"], op.path, prefix + "-idle"
                    ),
                }
                if args["scenario"] != "idle":
                    sample["interaction"] = workload(
                        device, artifact, args["scenario"], op.path, prefix
                    )
                pid, _ = cpu_ticks(device, artifact["package"])
                errors = device.adb(
                    "logcat",
                    "-d",
                    "--pid",
                    pid,
                    "-v",
                    "brief",
                    "-s",
                    "Ink:E",
                    "AndroidRuntime:E",
                    "*:S",
                )
                (op.path / f"{prefix}-errors.txt").write_text(errors)
                if re.search(r"^[EF]/", errors, re.MULTILINE):
                    raise RuntimeError(
                        "App reported runtime errors; inspect the saved error log"
                    )
                device.foreground(artifact["package"])
                device.thermal()
                device.screenshot(op.path / f"{prefix}.png")
                result["samples"].append(sample)
                write_json(op.path / "result.json", result)
                device.shell("am", "force-stop", artifact["package"])
        summary = {}
        for variant in builds:
            samples = [
                s["idle"]["pssKiB"]
                for s in result["samples"]
                if s["variant"] == variant
            ]
            summary[variant] = {
                "medianMiB": statistics.median(samples) / 1024,
                "minMiB": min(samples) / 1024,
                "maxMiB": max(samples) / 1024,
            }
            if args["scenario"] != "idle":
                interactions = [
                    s["interaction"]
                    for s in result["samples"]
                    if s["variant"] == variant
                ]
                summary[variant]["cpuMs"] = statistics.median(
                    s["cpuMs"] for s in interactions
                )
                summary[variant]["postInteractionMiB"] = (
                    statistics.median(s["memory"]["pssKiB"] for s in interactions)
                    / 1024
                )
                if args["scenario"] == "scroll":
                    summary[variant]["continuousP99Ms"] = statistics.median(
                        s["continuous"]["presentP99Ms"] for s in interactions
                    )
        saving = summary["baseline"]["medianMiB"] - summary["candidate"]["medianMiB"]
        overlap = max(summary[v]["minMiB"] for v in builds) <= min(
            summary[v]["maxMiB"] for v in builds
        )
        summary["savingMiB"] = saving
        summary["interpretation"] = (
            "Inconclusive: small difference, overlapping ranges, or fewer than three rounds."
            if args["rounds"] < 3 or abs(saving) < 0.25 or overlap
            else "Separated sample ranges; repeat before generalising beyond this device/session."
        )
        result["summary"] = summary
        write_json(op.path / "result.json", result)
        report = [
            "# Paired Ink experiment",
            "",
            f"Fixture: `{args['app']}`. Scenario: `{args['scenario']}`. Rounds: {args['rounds']}.",
            "",
            "| Idle PSS | Median | Range |",
            "| --- | ---: | ---: |",
        ]
        for variant in builds:
            s = summary[variant]
            report.append(
                f"| {variant} | {s['medianMiB']:.2f} MiB | {s['minMiB']:.2f}–{s['maxMiB']:.2f} MiB |"
            )
        report += [
            "",
            f"Baseline minus candidate: {saving:.2f} MiB. {summary['interpretation']}",
            "",
            "Idle samples follow a fresh launch and two-second settle. Post-interaction memory is recorded separately in [result.json](result.json).",
            "CPU/frame samples, when requested, are in the same file. Tap intervals include input-command gaps.",
            "Sample acceptance requires the expected foreground app and thermal status 0. Screenshots and Android dumps are retained beside this report.",
            "Source snapshots and APK hashes are recorded in the linked experiment manifests; these measurements do not establish peak memory.",
        ]
        if args["scenario"] != "idle":
            report += [
                "",
                "| Interaction median | Baseline | Candidate |",
                "| --- | ---: | ---: |",
            ]
            for label, key, unit in [
                ("CPU time", "cpuMs", "ms"),
                ("PSS after interaction", "postInteractionMiB", "MiB"),
            ]:
                report.append(
                    f"| {label} | {summary['baseline'][key]:.2f} {unit} | {summary['candidate'][key]:.2f} {unit} |"
                )
            if args["scenario"] == "scroll":
                report.append(
                    f"| Continuous-scroll p99 | {summary['baseline']['continuousP99Ms']:.0f} ms | {summary['candidate']['continuousP99Ms']:.0f} ms |"
                )
        for variant in builds:
            report += [
                "",
                f"[{variant} manifest](../{args[variant]}/manifest.json). Recorded flags: `{json.dumps(builds[variant][1]['flags'], sort_keys=True)}`.",
            ]
        (op.path / "report.md").write_text("\n".join(report) + "\n")
        return result
    finally:
        op.step("Restoring device settings and removing temporary apps")
        device.release()


def explain_memory(op, args):
    device = Device(args["serial"])
    package = args["package"]
    pid, _ = cpu_ticks(device, package)
    result = {
        "android": memory_sample(device, package, op.path, "memory"),
        "ink": None,
        "notes": [
            "Android PSS and Ink logical capacities overlap; never add them together."
        ],
    }
    log = device.adb("logcat", "-d", "--pid", pid, "-v", "brief", "-s", "Ink:I", "*:S")
    (op.path / "ink-log.txt").write_text(log)
    for line in log.splitlines():
        if "InkMemory " in line:
            result["ink"] = json.loads(line.split("InkMemory ", 1)[1])
    if result["ink"] is None:
        result["notes"].append(
            "No InkMemory record for this PID. Build with INK_MEMORY_DIAGNOSTICS=1 to collect engine counters; normal release builds contain no such instrumentation."
        )
    else:
        result["notes"].append(
            "Ink counters describe the latest logged rendered frame, which may precede the Android sample."
        )
    result["pid"] = pid
    if cpu_ticks(device, package)[0] != pid:
        raise RuntimeError("App restarted during memory capture; sample rejected")
    write_json(op.path / "result.json", result)
    return result


def inspect_image(op, args):
    from PIL import Image, ImageChops, ImageStat, __version__

    source = Path(args["file"]).resolve()
    with Image.open(source) as opened:
        image = opened.convert("RGBA")
    region = [0, 0, image.width, image.height]
    if args.get("region"):
        x, y, width, height = map(int, args["region"].split(","))
        if (
            min(x, y) < 0
            or min(width, height) <= 0
            or x + width > image.width
            or y + height > image.height
        ):
            raise ValueError("Region must be x,y,width,height inside the image")
        region = [x, y, x + width, y + height]
    cropped = image.crop(region)
    cropped.save(op.path / "crop.png")
    rgb = cropped.convert("RGB")
    red, green, blue = rgb.split()
    minimum = ImageChops.darker(red, ImageChops.darker(green, blue))
    maximum = ImageChops.lighter(red, ImageChops.lighter(green, blue))
    pixel_count = cropped.width * cropped.height
    result = {
        "file": str(source),
        "sha256": digest(source),
        "pillow": __version__,
        "size": list(image.size),
        "regionXYXY": region,
        "regionPixels": pixel_count,
        "brightPixels": sum(minimum.histogram()[150:]),
        "colouredPixels": sum(ImageChops.subtract(maximum, minimum).histogram()[21:]),
        "meanRGB": ImageStat.Stat(rgb).mean,
    }
    if args.get("compare"):
        other_path = Path(args["compare"]).resolve()
        with Image.open(other_path) as opened:
            other = opened.convert("RGBA")
        if other.size != image.size:
            raise ValueError(
                "Image dimensions differ; explicit resizing is required before comparing"
            )
        delta = ImageChops.difference(cropped, other.crop(region))
        channels = delta.split()
        mask = channels[0]
        for channel in channels[1:]:
            mask = ImageChops.lighter(mask, channel)
        threshold = args["threshold"]
        changed = mask.point(lambda p: 255 if p > threshold else 0)
        changed.save(op.path / "diff.png")
        count = changed.histogram()[255]
        result["comparison"] = {
            "file": str(other_path),
            "sha256": digest(other_path),
            "threshold": threshold,
            "changedPixels": count,
            "changedPercent": count * 100 / pixel_count,
            "changedBoundsInCrop": changed.getbbox(),
            "meanAbsoluteRGBA": ImageStat.Stat(delta).mean,
        }
    if args.get("ocr"):
        if sys.platform == "darwin":
            result["text"] = json.loads(
                run(
                    [
                        "swift",
                        ROOT / "scripts/agent_tools_ocr.swift",
                        op.path / "crop.png",
                    ],
                    timeout=120,
                )
            )
        elif shutil.which("tesseract"):
            result["text"] = run(["tesseract", op.path / "crop.png", "stdout"])
        else:
            raise RuntimeError("OCR needs macOS Vision/Swift or tesseract on PATH")
    write_json(op.path / "result.json", result)
    return result


def worker(path):
    op = Operation(path)

    def interrupted(_number, _frame):
        raise KeyboardInterrupt("Cancelled")

    signal.signal(signal.SIGTERM, interrupted)
    op.update(status="running", pid=os.getpid())
    try:
        args = read_json(op.path / "request.json")
        from agent_stress import stress

        result = {
            "experiment": build,
            "bench": compare,
            "stress": lambda op, args: stress(sys.modules[__name__], op, args),
            "memory": explain_memory,
            "image": inspect_image,
        }[args["command"]](op, args)
        op.update(
            status="complete",
            result="manifest.json"
            if args["command"] == "experiment"
            else "result.json",
        )
        emit(
            {
                "id": op.state["id"],
                "status": "complete",
                "summary": result.get("summary"),
            }
        )
    except (Exception, KeyboardInterrupt) as error:  # noqa: BLE001 -- persist failures at the worker boundary
        op.update(
            status="cancelled" if isinstance(error, KeyboardInterrupt) else "failed",
            error=str(error) or type(error).__name__,
        )
        emit(op.state)
        return 1
    return 0


def positive(value):
    value = int(value)
    if value < 1:
        raise argparse.ArgumentTypeError("Must be positive")
    return value


def parser():
    cli = argparse.ArgumentParser(description=__doc__)
    commands = cli.add_subparsers(dest="command", required=True)
    experiment = commands.add_parser(
        "experiment", help="Build an isolated, hashed source snapshot"
    )
    source = experiment.add_mutually_exclusive_group(required=True)
    source.add_argument("--ref", help="Existing git commit or ref")
    source.add_argument("--working-tree", action="store_true")
    experiment.add_argument(
        "--app",
        action="append",
        required=True,
        help="Repository-relative app directory; repeatable",
    )
    experiment.add_argument(
        "--env",
        action="append",
        default=[],
        help="INK_SPLIT_WEB, INK_BENCHMARK, INK_PRESENTATION_TIMING or INK_MEMORY_DIAGNOSTICS, each 0 or 1",
    )
    bench = commands.add_parser(
        "bench", help="Alternate baseline/candidate APKs on a reserved device"
    )
    for key in ["baseline", "candidate", "app"]:
        bench.add_argument("--" + key)
    bench.add_argument("--serial", required=True)
    bench.add_argument("--comparison", help="JSON mapping ink, expo and light-sdk to counter APK paths")
    bench.add_argument("--rounds", type=positive, default=3)
    bench.add_argument(
        "--scenario", choices=["idle", "counter", "scroll"], default="idle"
    )
    stress = commands.add_parser("stress", help="Check repeated Ink journeys and collect renderer benchmarks")
    stress.add_argument("--baseline", required=True, help="Completed ink-stress experiment ID")
    stress.add_argument("--candidate", help="Optional experiment to compare in alternating order")
    stress.add_argument("--serial", required=True)
    stress.add_argument("--rounds", type=positive, default=1)
    stress.add_argument("--cycles", type=positive, default=10)
    stress.add_argument("--idle-seconds", type=positive, default=10)
    memory = commands.add_parser(
        "memory", help="Explain live foreground process memory; read-only"
    )
    memory.add_argument("--serial", required=True)
    memory.add_argument("--package", required=True)
    image = commands.add_parser(
        "image", help="Inspect actual pixels, compare screenshots and recognise text"
    )
    image.add_argument("file")
    image.add_argument("--compare")
    image.add_argument("--region", help="x,y,width,height")
    image.add_argument(
        "--threshold", type=int, choices=range(256), default=0, metavar="0..255"
    )
    image.add_argument("--ocr", action="store_true")
    for command in [experiment, bench, stress, memory, image]:
        command.add_argument(
            "--background",
            action="store_true",
            help="Return the operation ID immediately",
        )
    device = commands.add_parser(
        "device", help="Cooperative reservations shared across checkouts"
    )
    actions = device.add_subparsers(dest="action", required=True)
    for name in ["acquire", "status", "release", "run"]:
        action = actions.add_parser(name)
        action.add_argument("--serial", required=True)
        action.add_argument("--full", action="store_true")
        if name == "acquire":
            action.add_argument("--owner", required=True)
        if name in {"release", "run"}:
            action.add_argument("--token", required=True)
        if name == "run":
            action.add_argument("--experiment", required=True)
            action.add_argument("--app", required=True)
    for name in ["status", "result", "cancel", "clean", "wait"]:
        action = commands.add_parser(name)
        action.add_argument("id")
        if name in {"status", "result"}:
            action.add_argument(
                "--full", action="store_true", help="Return the complete stored JSON"
            )
        if name == "wait":
            action.add_argument(
                "--after", help="Cursor from a previous status/wait response"
            )
            action.add_argument(
                "--timeout", type=int, choices=range(61), default=30, metavar="0..60"
            )
    listing = commands.add_parser("list", help="List recent operations, newest first")
    listing.add_argument("--limit", type=positive, default=5)
    listing.add_argument("--offset", type=int, default=0)
    listing.add_argument("--full", action="store_true")
    return cli


def main():
    if len(sys.argv) == 3 and sys.argv[1] == "_run":
        return worker(sys.argv[2])
    args = vars(parser().parse_args())
    command = args["command"]
    if command == "device":
        device = Device(args["serial"])
        if args["action"] == "acquire":
            session = device.acquire(args["owner"])
            emit(
                session
                if args["full"]
                else {key: session[key] for key in ["serial", "token", "owner"]}
            )
        elif args["action"] == "status":
            if device.path.exists():
                session = read_json(device.path / "session.json")
                emit(
                    session
                    if args["full"]
                    else {
                        "reserved": True,
                        **{key: session[key] for key in ["serial", "token", "owner"]},
                    }
                )
            else:
                emit({"serial": device.serial, "reserved": False})
        else:
            device.attach(args["token"])
            if args["action"] == "run":
                path, _, artifact = load_build(args["experiment"], args["app"])
                device.install(
                    artifact["package"], path / artifact["file"], artifact["sha256"]
                )
                launch = device.start(artifact["package"], artifact["component"])
                emit(
                    {
                        "serial": device.serial,
                        "package": artifact["package"],
                        **(
                            {"launch": launch}
                            if args["full"]
                            else {
                                "launchMs": int(
                                    re.search(r"TotalTime:\s+(\d+)", launch)[1]
                                )
                                if re.search(r"TotalTime:\s+(\d+)", launch)
                                else None
                            }
                        ),
                    }
                )
            else:
                device.release()
                emit({"serial": device.serial, "released": True})
        return 0
    if command == "list":
        if args["offset"] < 0:
            raise ValueError("Offset must be non-negative")
        states = sorted(
            (read_json(p) for p in STORE.glob("*/status.json")),
            key=lambda s: s["created"],
            reverse=True,
        )
        end = args["offset"] + args["limit"]
        page = states[args["offset"] : end]
        emit(
            {
                "operations": page
                if args["full"]
                else [{key: state[key] for key in ["id", "status"]} for state in page],
                "nextOffset": end if end < len(states) else None,
            }
        )
        return 0
    if command == "wait":
        emit(wait_operation(args["id"], args["after"], args["timeout"]))
        return 0
    if command == "result":
        path = operation_path(args["id"])
        state = read_json(path / "status.json")
        if not state.get("result"):
            emit(operation_view(state))
        else:
            result = read_json(path / state["result"])
            emit(result if args["full"] else operation_view(state))
        return 0
    if command in {"status", "cancel", "clean"}:
        path = operation_path(args["id"])
        state = read_json(path / "status.json")
        if command == "cancel":
            if state["status"] not in {"queued", "running"}:
                raise RuntimeError("Operation is already finished")
            pid = state.get("pid")
            process = (
                run(["ps", "-p", str(pid), "-o", "command="], check=False)
                if pid
                else ""
            )
            if f"_run {path}" not in process or os.getpgid(pid) != pid:
                raise RuntimeError(
                    "No matching worker; inspect status and device reservation before recovery"
                )
            os.killpg(pid, signal.SIGTERM)
            emit({"id": args["id"], "cancellationRequested": True})
        elif command == "clean":
            if state["status"] in {"queued", "running"}:
                raise RuntimeError("Cannot clean an active operation")
            if (path / "workspace").exists():
                shutil.rmtree(path / "workspace")
            emit({"id": args["id"], "workspaceRemoved": True, "evidenceRetained": True})
        else:
            emit(operation_view(state, args["full"]))
        return 0
    if command == "bench" and args.get("comparison"):
        args["comparison"] = str(Path(args["comparison"]).resolve())
    if command == "image":
        for key in ["file", "compare"]:
            if args.get(key):
                args[key] = str(Path(args[key]).resolve())
    identifier = f"{command}-{time.strftime('%Y%m%d-%H%M%S')}-{uuid.uuid4().hex[:8]}"
    path = operation_path(identifier)
    path.mkdir(parents=True)
    write_json(path / "request.json", args)
    write_json(
        path / "status.json",
        {
            "id": identifier,
            "kind": command,
            "status": "queued",
            "created": time.time(),
            "directory": str(path),
        },
    )
    with (path / "operation.log").open("w") as log:
        process = subprocess.Popen(
            [sys.executable, __file__, "_run", str(path)],
            stdout=log,
            stderr=log,
            start_new_session=True,
        )
    if args["background"]:
        emit({"id": identifier, "status": "queued"})
        return 0
    try:
        code = process.wait()
    except KeyboardInterrupt:
        os.killpg(process.pid, signal.SIGTERM)
        code = process.wait()
    emit(operation_view(read_json(path / "status.json")))
    return code


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as error:  # noqa: BLE001 -- the CLI returns structured failures
        emit({"status": "error", "error": str(error)})
        sys.exit(1)
