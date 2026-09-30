import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "../..");
const packageName = "com.vandam.ink.contracttests";
const args = process.argv.slice(2);
if (args.length !== 2 || args[0] !== "--serial" || !/^[\w.:-]+$/.test(args[1]!)) {
  console.error("Usage: bun test/android/run.ts --serial <adb-device-serial>");
  process.exit(1);
}
const serial = args[1]!;
const output = resolve(root, ".test-output/android", new Date().toISOString().replaceAll(":", "-"));
await mkdir(output, { recursive: true });

async function command(argv: string[], name: string, timeoutMs = 30_000, env = process.env) {
  const process = Bun.spawn(argv, { cwd: root, env, stdout: "pipe", stderr: "pipe" });
  const timeout = setTimeout(() => process.kill(), timeoutMs);
  const [stdout, stderr, code] = await Promise.all([
    new Response(process.stdout).text(), new Response(process.stderr).text(), process.exited,
  ]);
  clearTimeout(timeout);
  await writeFile(resolve(output, `${name}.log`), stdout + stderr);
  if (code !== 0) throw new Error(`${name} failed (exit ${code}). ${stdout}${stderr}\nLog: ${output}/${name}.log`);
  return stdout;
}

let locked = false;
let installed = false;
const lock = resolve(root, ".test-output", `android-${serial}.lock`);
try {
  if (!Bun.which("adb")) throw new Error("Android tests require adb on PATH (Android SDK platform-tools).");
  const sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT ?? resolve(homedir(), "Library/Android/sdk");
  if (!existsSync(resolve(sdk, "platforms/android-36/android.jar"))) {
    throw new Error("Android tests require SDK platform 36; set ANDROID_HOME to your Android SDK.");
  }
  let javaHome = process.env.JAVA_HOME;
  if (!javaHome && process.platform === "darwin") {
    const candidate = Bun.spawnSync(["/usr/libexec/java_home", "-v", "17"], { stdout: "pipe", stderr: "pipe" });
    if (candidate.exitCode === 0) javaHome = candidate.stdout.toString().trim();
  }
  if (!javaHome || !existsSync(resolve(javaHome, "bin/java"))) {
    throw new Error("Android tests require JDK 17 or 21; set JAVA_HOME. Gradle 8.14.3 does not support JDK 25.");
  }
  const version = await command([resolve(javaHome, "bin/java"), "-version"], "java-version");
  const versionLog = version + await readFile(resolve(output, "java-version.log"), "utf8");
  if (!/version "(17|21)\./.test(versionLog)) throw new Error("Set JAVA_HOME to JDK 17 or 21 for Android tests.");
  const devices = await command(["adb", "devices"], "devices");
  if (!devices.split("\n").some(line => line.trim() === `${serial}\tdevice`)) {
    throw new Error(`Device ${serial} is unavailable or unauthorised. Start emulator -avd Light_Phone_III -writable-system, then pass its serial explicitly.`);
  }
  if ((await command(["adb", "-s", serial, "shell", "getprop", "sys.boot_completed"], "boot-status")).trim() !== "1") {
    throw new Error(`Device ${serial} has not completed boot. Retry once sys.boot_completed is 1.`);
  }
  const api = Number((await command(["adb", "-s", serial, "shell", "getprop", "ro.build.version.sdk"], "device-api")).trim());
  if (api < 34) throw new Error(`Android tests require API 34 or later; ${serial} runs API ${api}.`);
  try { await mkdir(lock); locked = true; }
  catch { throw new Error(`Android tests already own ${serial} (${lock}). Remove this directory only if that run has stopped.`); }
  const packages = await command(["adb", "-s", serial, "shell", "pm", "list", "packages", packageName], "existing-package");
  if (packages.split("\n").some(line => line.trim() === `package:${packageName}`)) {
    throw new Error(`Refusing to replace existing fixture ${packageName}. Uninstall it explicitly before rerunning.`);
  }
  console.log(`Building Android fixture; logs: ${output}`);
  await command([resolve(root, "platform/android/gradlew"), "-p", "test/fixtures/android", "assembleDebug", "--console=plain"], "build", 300_000,
    { ...process.env, JAVA_HOME: javaHome, ANDROID_HOME: sdk });
  await command(["adb", "-s", serial, "install", resolve(root, "test/fixtures/android/build/outputs/apk/debug/InkAndroidContracts-debug.apk")], "install");
  installed = true;
  console.log(`Running native Android contracts on ${serial}; real idle expiry takes 61 seconds.`);
  const result = await command(["adb", "-s", serial, "shell", "am", "instrument", "-w", "-r", `${packageName}/com.vandam.ink.ContractInstrumentation`], "instrumentation", 180_000);
  console.log(result);
  const summaryLine = result.split("\n").find(line => line.startsWith("INSTRUMENTATION_RESULT: ink.summary="));
  if (!summaryLine) throw new Error(`Instrumentation did not produce a test summary. See ${output}/instrumentation.log`);
  const summary = JSON.parse(summaryLine.slice("INSTRUMENTATION_RESULT: ink.summary=".length)) as { passed: number; failures: string[] };
  if (!Array.isArray(summary.failures) || summary.passed + summary.failures.length !== 15) throw new Error("Instrumentation returned an incomplete test summary.");
  if (summary.failures.length) throw new Error(`${summary.failures.length} Android contract(s) failed. See ${output}/instrumentation.log`);
  console.log(`${summary.passed} Android contracts passed.`);
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
} finally {
  if (installed) {
    try { await command(["adb", "-s", serial, "uninstall", packageName], "uninstall"); }
    catch (error) { console.error(`Fixture cleanup failed: ${error}`); process.exitCode = 1; }
  }
  if (locked) await rm(lock, { recursive: true });
}
