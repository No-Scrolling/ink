import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const [apkDirectory, outputDirectory] = process.argv.slice(2);
if (!apkDirectory || !outputDirectory) throw new Error("Usage: bun measure.ts <APK directory> <new output directory>");
const serial = process.env.ANDROID_SERIAL ?? "LP3LHMA531900140";
mkdirSync(outputDirectory, { recursive: true });
function adb(...args: string[]) {
  const result = Bun.spawnSync(["adb", "-s", serial, ...args]);
  if (result.exitCode) throw new Error(result.stderr.toString());
  return result.stdout.toString();
}
for (let round = 1; round <= 3; round++) {
  for (const fixture of ["counter", "scroll"]) {
    const pkg = `com.vandam.benchmark.ink.${fixture}`;
    for (const variant of round % 2 ? ["control", "split"] : ["split", "control"]) {
      adb("install", "-r", resolve(apkDirectory, `${variant}-${fixture}.apk`));
      adb("shell", "input", "keyevent", "224");
      adb("shell", "wm", "dismiss-keyguard");
      adb("shell", "am", "force-stop", pkg);
      const launch = adb("shell", "am", "start", "-W", "-n", `${pkg}/com.vandam.ink.MainActivity`);
      await Bun.sleep(2000);
      const focus = adb("shell", "dumpsys", "window").split("\n").find(line => line.includes("mCurrentFocus="));
      if (!focus?.includes(pkg)) throw new Error(`Foreground changed: ${focus}`);
      const memory = adb("shell", "dumpsys", "meminfo", pkg);
      writeFileSync(resolve(outputDirectory, `${variant}-${fixture}-${round}.txt`), `${launch}\n${focus}\n${memory}`);
      console.log(variant, fixture, round, memory.match(/TOTAL PSS:.*$/m)?.[0]);
      adb("shell", "am", "force-stop", pkg);
    }
  }
}
