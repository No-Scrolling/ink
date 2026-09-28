"""Build and measure the real React-to-scene path without a UI."""

import json
import os
import platform
import shlex
import shutil
import statistics
import subprocess
from pathlib import Path


def source_hashes(tools, app="benchmarks/apps/ink-updates"):
    root = tools.ROOT
    inputs = [root / name for name in (".cargo/config.toml", "Cargo.toml", "Cargo.lock", "bun.lock", "package.json",
                                     "scripts/agent_headless.py", "scripts/agent_tools.py",
                                     "packages/ink/package.json", app + "/ink.toml", app + "/package.json")]
    for directory in ("benchmarks/headless", app + "/app", "crates/ink-core",
                      "crates/ink-runtime", "crates/ink-protocol", "crates/ink-compiler", "packages/ink/src"):
        inputs.extend(path for path in (root / directory).rglob("*") if path.is_file())
    if app == "benchmarks/apps/ink-native-controls":
        inputs.extend([root / "examples/light-template/app/examples/native-lists.tsx", root / "examples/light-template/app/examples/playing.tsx", root / "examples/light-template/assets/images/wallsocket.jpg"])
    return {str(path.relative_to(root)): tools.digest(path) for path in sorted(inputs)}


def quickjs_inputs(tools, directory):
    paths = [directory / "Cargo.toml", directory / "build.rs", directory / "quickjs.bind.h"]
    paths.extend((directory / "src").rglob("*.rs"))
    for suffix in ("*.c", "*.h"):
        paths.extend((directory / "quickjs").glob(suffix))
    return {str(path.relative_to(directory)): tools.digest(path) for path in sorted(paths)}


def verify_quickjs_build(tools, log, package, inputs):
    messages = [json.loads(line) for line in log.read_text().splitlines() if line.startswith("{")]
    builds = [message for message in messages if message.get("reason") == "build-script-executed"
              and message["package_id"] == package["id"]]
    if len(builds) != 1:
        raise RuntimeError("Expected one rquickjs-sys build record")
    output = Path(builds[0]["out_dir"])
    copied = {}
    for path in sorted(output.iterdir()):
        if path.suffix not in (".c", ".h"):
            continue
        name = "quickjs.bind.h" if path.name == "quickjs.bind.h" else "quickjs/" + path.name
        if name not in inputs:
            continue
        copied[name] = tools.digest(path)
        if copied[name] != inputs[name]:
            raise RuntimeError(f"Stale QuickJS build input: {name}. Rebuild rquickjs-sys before benchmarking.")
    if "quickjs/quickjs.c" not in copied or "quickjs/quickjs-opcode.h" not in copied:
        raise RuntimeError("QuickJS build did not expose its interpreter sources")
    return {"packageId": package["id"], "inputs": inputs, "compiledInputs": copied}


def headless(tools, op, args):
    root = tools.ROOT
    app = args.get("app", "benchmarks/apps/ink-updates")
    if app != "benchmarks/apps/ink-updates" and args.get("react_profile"):
        raise ValueError("React phase profiling requires the React update fixture")
    device = None
    if bool(args.get("serial")) != bool(args.get("token")):
        raise ValueError("Android headless runs require both --serial and --token")
    if args.get("serial"):
        if not args.get("ndk"):
            raise ValueError("Android headless runs require --ndk or ANDROID_NDK_HOME")
        if args["hyperfine"]:
            raise ValueError("Use internal timings on Android; Hyperfine is host-only")
        device = tools.Device(args["serial"])
        device.attach(args["token"])
        device.thermal()
    if args["hyperfine"] and not shutil.which("hyperfine"):
        raise RuntimeError("Install hyperfine before using --hyperfine")
    if args["hyperfine"] and args["runs"] < 2:
        raise ValueError("Hyperfine requires at least two runs")
    environment = dict(os.environ, CARGO_TARGET_DIR=str(root / "target"))
    source = source_hashes(tools, app)
    rust_host = next(line.removeprefix("host: ") for line in tools.run(["rustc", "-vV"]).splitlines()
                     if line.startswith("host: "))
    metadata_command = subprocess.run(["cargo", "metadata", "--locked", "--offline", "--format-version", "1",
                                       "--filter-platform", rust_host], cwd=root, capture_output=True, text=True, timeout=120)
    (op.path / "metadata.log").write_text(metadata_command.stderr)
    if metadata_command.returncode:
        raise RuntimeError(f"Cargo metadata failed: {metadata_command.stderr[-3000:]}")
    metadata = json.loads(metadata_command.stdout)
    package, = [package for package in metadata["packages"] if package["name"] == "rquickjs-sys"]
    quickjs_directory = Path(package["manifest_path"]).parent
    quickjs_source = quickjs_inputs(tools, quickjs_directory)
    quickjs_builds = {}
    op.step("Building the headless release harness")
    with (op.path / "build.log").open("w") as log:
        with (op.path / "host-build.jsonl").open("w") as build_log:
            tools.run(["cargo", "build", "--locked", "--release", "-p", "ink-headless",
                       "--message-format=json-render-diagnostics"],
                      cwd=root, env=environment, log=build_log, timeout=600)
        quickjs_builds["host"] = verify_quickjs_build(tools, op.path / "host-build.jsonl", package, quickjs_source)
        binary = op.path / "ink-headless"
        shutil.copy2(root / "target/release/ink-headless", binary)
        assets = op.path / "assets"
        op.step("Compiling the existing update fixture")
        tools.run([binary, "--prepare", root / app, "--assets", assets],
                  cwd=root, env=environment, log=log, timeout=120)
        if args.get("react_profile"):
            op.step("Instrumenting the diagnostic React bundle")
            tools.run(["bun", root / "benchmarks/headless/profile-react.mjs",
                       root / "benchmarks/apps/ink-updates", assets, args["react_profile"]],
                      cwd=root, env=environment, log=log, timeout=120)
        if device:
            op.step("Building the Android CPU harness")
            ndk = Path(args["ndk"]).expanduser().resolve()
            prebuilts = list((ndk / "toolchains/llvm/prebuilt").glob("*/sysroot"))
            if len(prebuilts) != 1:
                raise ValueError("The NDK must contain one host toolchain sysroot")
            sysroot = prebuilts[0]
            environment["ANDROID_NDK_HOME"] = str(ndk)
            environment["BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android"] = shlex.join(
                [f"--sysroot={sysroot}", f"-I{sysroot}/usr/include/aarch64-linux-android"])
            with (op.path / "android-build.jsonl").open("w") as build_log:
                tools.run(["cargo", "ndk", "-t", "arm64-v8a", "--platform", "34",
                           "build", "--locked", "--release", "-p", "ink-headless",
                           "--message-format=json-render-diagnostics"],
                          cwd=root, env=environment, log=build_log, timeout=600)
            quickjs_builds["android"] = verify_quickjs_build(tools, op.path / "android-build.jsonl", package, quickjs_source)
            shutil.copy2(root / "target/aarch64-linux-android/release/ink-headless", binary)
    command = [str(binary), "--assets", str(assets), "--iterations", str(args["iterations"]),
               "--warmup", str(args["warmup"])]
    if app == "benchmarks/apps/ink-native-controls":
        command.append("--native-controls")
    if app == "benchmarks/apps/ink-list":
        command.append("--list-compat")
    if args.get("react_profile"):
        command.append("--react-profile")
    op.step("Measuring native input to scene layout")
    if device:
        remote = "/data/local/tmp/ink-" + op.state["id"]
        device.attach(args["token"])
        device.shell("mkdir", remote)
        try:
            device.adb("push", binary, remote + "/ink-headless")
            device.adb("push", assets, remote + "/assets")
            device.shell("chmod", "700", remote + "/ink-headless")
            device.thermal()
            result = json.loads(device.shell(remote + "/ink-headless", "--assets",
                                            remote + "/assets", *command[3:], timeout=120))
            device.thermal()
        finally:
            device.shell("rm", "-rf", remote)
        result["execution"] = {"kind": "android-shell", "serial": device.serial}
        result["note"] += " Runs as Android shell, not an Activity; app scheduling and frame submission are not measured."
    else:
        result = json.loads(tools.run(command, cwd=root, timeout=120))
    if args.get("react_profile"):
        if len(result.get("reactProfiles", [])) != args["iterations"]:
            raise RuntimeError("Missing diagnostic React timings")
        result["reactProfileMode"] = args["react_profile"]
        profiles = result["reactProfiles"]
        sample_field = {"jsx": "jsxSamples", "host": "hostUpdateSamples"}.get(args["react_profile"])
        if sample_field and not sum(profile[sample_field] for profile in profiles):
            raise RuntimeError("The requested React sampling hook did not run")
        result["reactProfileMeans"] = {
            name: statistics.mean(profile[name] for profile in profiles)
            for name in profiles[0]
        }
        result["reactProfileMeans"]["reconcilerWorkMs"] = statistics.mean(
            profile["workLoopMs"] - profile["componentMs"] for profile in profiles)
        result["reactProfileMeans"]["beforeCommitOverheadMs"] = statistics.mean(
            profile["renderMs"] - profile["workLoopMs"] for profile in profiles)
        for stem, label in [("jsx", "Jsx"), ("hostUpdate", "HostUpdate")]:
            sampled = sum(profile[stem + "Samples"] for profile in profiles)
            if sampled:
                estimate = (sum(profile[stem + "MeasuredMs"] for profile in profiles) / sampled
                            * statistics.mean(profile[stem + "Calls"] for profile in profiles))
                result["reactProfileMeans"]["estimated" + label + "Ms"] = estimate
                result["reactProfileMeans"]["estimated" + label + "MinusClockMs"] = (
                    estimate - statistics.mean(profile["clockPairMs"] * profile[stem + "Calls"] for profile in profiles))
    result["fixture"] = app
    if app == "benchmarks/apps/ink-views":
        result["note"] += " Native bindings fixture: React mounts the page; measured value updates bypass React reconciliation. This is not the unchanged React workload."
    if source != source_hashes(tools, app):
        raise RuntimeError("Benchmark sources changed during the run; retry")
    if quickjs_source != quickjs_inputs(tools, quickjs_directory):
        raise RuntimeError("QuickJS sources changed during the run; retry")
    result["provenance"] = {
        "inputs": source,
        "quickjs": quickjs_builds,
        "binarySha256": tools.digest(binary),
        "bundleSha256": tools.digest(assets / "app.js"),
        "iconsSha256": tools.digest(assets / "ink-icons-v1.bin"),
        "host": platform.platform(),
        "buildEnvironment": {name: value for name, value in environment.items()
                             if name in ("CFLAGS", "RUSTFLAGS", "CC")
                             or name.startswith(("CFLAGS_", "CARGO_PROFILE_RELEASE_"))},
        "rust": tools.run(["rustc", "--version"]).strip(),
        "bun": tools.run(["bun", "--version"]).strip(),
    }
    if device:
        result["provenance"]["ndk"] = (ndk / "source.properties").read_text()
    repeat = ["scripts/agent-tools", "headless", "--app", app, "--serial", args.get("serial"),
              "--token", "RESERVATION_TOKEN", "--ndk", str(ndk), *command[3:]] if device else command
    if device and args.get("react_profile"):
        repeat[-1:] = ["--react-profile", args["react_profile"]]
    result["command"] = shlex.join(repeat)
    if args["hyperfine"]:
        op.step("Running Hyperfine against the prepared harness")
        with (op.path / "hyperfine.log").open("w") as log:
            tools.run(["hyperfine", "--warmup", "2", "--runs", str(args["runs"]),
                       "--export-json", op.path / "hyperfine.json", result["command"]],
                      cwd=root, log=log, timeout=600)
        result["hyperfine"] = tools.read_json(op.path / "hyperfine.json")["results"][0]
    tools.write_json(op.path / "result.json", result)
    rows = ["# Headless React-to-scene benchmark", "", result["note"], "",
            f"Measured updates: {args['iterations']}; discarded warm-up updates: {args['warmup']}.", "",
            "| Stage | Mean (ms) | Median (ms) | p95 (ms) |", "| --- | ---: | ---: | ---: |"]
    for stage, timing in result["milliseconds"].items():
        rows.append(f"| {stage} | {timing['mean']:.3f} | {timing['median']:.3f} | {timing['p95']:.3f} |")
    if result["waterfallMs"]:
        rows.extend(["", "## Whole-suite breakdown", "",
                     "Accumulated wall time in this instrumented run, from loading assets through runtime shutdown. "
                     "Excludes process launch, argument parsing and final report serialisation/output. "
                     "Hyperfine runs are separate measurements.", "",
                     "| Work | Total (ms) | Share |", "| --- | ---: | ---: |"])
        for stage, elapsed in result["waterfallMs"].items():
            rows.append(f"| {stage} | {elapsed:.3f} | {100 * elapsed / result['suiteMs']:.1f}% |")
        rows.append(f"| **Total accounted** | **{result['suiteMs']:.3f}** | **100%** |")
    rows.extend(["", "Checks: " + ", ".join(result["checks"]) + ".", "",
                 "Run the prepared binary again:", "", "```sh", result["command"], "```", ""])
    if "reactProfileMeans" in result:
        rows.extend(["## Diagnostic React timings", "",
                     "Instrumented attribution only; use uninstrumented runs for speed comparisons. "
                     "Component time is inside render time; JSX estimates are inside component time; host-update estimates are inside commit time. "
                     "These must not be added together. Raw samples are in result.json.", "",
                     "| Measurement | Mean |", "| --- | ---: |"])
        for name, value in result["reactProfileMeans"].items():
            rows.append(f"| {name} | {value:.6f} |")
    if "hyperfine" in result:
        timing = result["hyperfine"]
        rows.append(f"Hyperfine whole-process mean: {timing['mean']:.3f} s ± {timing['stddev']:.3f} s (standard deviation).")
    (op.path / "report.md").write_text("\n".join(rows) + "\n")
    return result
