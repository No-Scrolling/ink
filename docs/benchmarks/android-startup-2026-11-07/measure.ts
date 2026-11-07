import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

// Both APKs must use this isolated package and the same signing certificate.
const applicationId = "com.vandam.ink.startupprobe";
const activity = `${applicationId}/com.vandam.ink.MainActivity`;
const [serial, baseline, candidate, output] = process.argv.slice(2);
if (!serial || !baseline || !candidate || !output) {
  throw Error("Usage: bun measure.ts SERIAL BASELINE_APK CANDIDATE_APK OUTPUT_DIRECTORY");
}
await mkdir(output, { recursive: true });
async function adb(...args: string[]) {
  const child = Bun.spawn(["adb", "-s", serial, ...args], { stdout: "pipe", stderr: "pipe" });
  const [code, stdout, stderr] = await Promise.all([child.exited, new Response(child.stdout).text(), new Response(child.stderr).text()]);
  if (code) throw Error(`adb ${args.join(" ")}: ${stdout}${stderr}`);
  return stdout;
}
const samplesPath = resolve(output, "samples.json");
const previous = await Bun.file(samplesPath).exists() ? await Bun.file(samplesPath).json() : undefined;
if (previous && (previous.serial !== serial || previous.applicationId !== applicationId)) throw Error("Output belongs to another device or app");
const results = previous?.samples ?? [];
const discards = previous?.discards ?? [];
async function save() {
  await writeFile(samplesPath, JSON.stringify({ serial, applicationId, samples: results, discards }, null, 2) + "\n");
}
async function foreground(pair: number, variant: string, stage: string, launch = "") {
  const activities = await adb("shell", "dumpsys", "activity", "activities");
  const resumed = activities.split("\n").filter((line) => /mResumedActivity|topResumedActivity/.test(line));
  if (resumed.some((line) => line.includes(applicationId))) return true;
  const discard = { pair, variant, stage, resumed };
  discards.push(discard);
  await writeFile(resolve(output, `discard-${discards.length}.txt`), `${JSON.stringify(discard)}\n${launch}`);
  await save();
  console.log("Discarded", JSON.stringify(discard));
  if (discards.length >= 60) throw Error("Persistent foreground contention; retain indicative samples and the discard count");
  return false;
}
for (let pair = 0; pair < 15; pair++) {
  for (const variant of ["before", "after"]) {
    if (results.some((sample: { pair: number; variant: string }) => sample.pair === pair && sample.variant === variant)) continue;
    const apk = resolve(variant === "before" ? baseline : candidate);
    const install = await adb("install", "-r", apk);
    while (true) {
      // Establish the probe as foreground before the shared emulator's measured cold launch.
      if (serial.startsWith("emulator-")) {
        await adb("shell", "am", "start", "-W", "-n", activity);
        if (!await foreground(pair, variant, "before launch")) continue;
      }
      const token = `startup-${Date.now()}-${pair}-${variant}`;
      const launch = await adb("shell", `log -t InkStartupProbe '${token}'; am start -W -S -n ${activity}`);
      if (!await foreground(pair, variant, "after launch", launch)) continue;
      const totalTime = Number(launch.match(/^TotalTime: (\d+)/m)?.[1]);
      if (!totalTime || !launch.includes("Status: ok")) throw Error(launch);
      const pid = (await adb("shell", "pidof", applicationId)).trim();
      if (!/^\d+$/.test(pid)) throw Error(`Unexpected probe PID: ${pid}`);
      await Bun.sleep(2000);
      if (!await foreground(pair, variant, "after settle", launch)) continue;
      const logs = await adb("logcat", "-d", "-v", "epoch", "-t", "5000");
      const begin = logs.split("\n").findLast((line) => line.includes(token));
      if (!begin) throw Error("Missing launch marker");
      const beginTime = Number(begin.trim().split(/\s+/)[0]);
      const processLogs = logs.split("\n").filter((line) => {
        const fields = line.trim().split(/\s+/);
        return fields[1] === pid && Number(fields[0]) >= beginTime;
      });
      const ready = processLogs.find((line) => line.includes("JavaScript app ready"));
      const presented = processLogs.find((line) => /Presented id=\d+ scene=[1-9]\d*/.test(line));
      if (!ready || !presented) throw Error(`Missing content markers:\n${processLogs.join("\n")}`);
      const elapsed = (line: string) => (Number(line.trim().split(/\s+/)[0]) - beginTime) * 1000;
      const memory = await adb("shell", "dumpsys", "meminfo", applicationId);
      const pss = Number(memory.match(/TOTAL PSS:\s+(\d+)/)?.[1]);
      const rss = Number(memory.match(/TOTAL RSS:\s+(\d+)/)?.[1]);
      if (!pss || !rss) throw Error(memory);
      if (!await foreground(pair, variant, "after memory", launch)) continue;
      if (pair === 0) {
        const screenshot = Bun.spawn(["adb", "-s", serial, "exec-out", "screencap", "-p"], { stdout: "pipe", stderr: "pipe" });
        const bytes = await new Response(screenshot.stdout).arrayBuffer();
        if (await screenshot.exited) throw Error("Screenshot failed");
        if (!await foreground(pair, variant, "after screenshot", launch)) continue;
        await writeFile(resolve(output, `${variant}.png`), new Uint8Array(bytes));
      }
      const prefix = `${pair.toString().padStart(2, "0")}-${variant}`;
      await writeFile(resolve(output, `${prefix}.txt`), `${install}\n${launch}\n${begin}\n${processLogs.join("\n")}\n${memory}`);
      const sample = { pair, variant, pid, totalTimeMs: totalTime, javascriptReadyMs: elapsed(ready), firstContentMs: elapsed(presented), pssKb: pss, rssKb: rss };
      results.push(sample);
      await save();
      console.log(serial, JSON.stringify(sample));
      await adb("shell", "am", "force-stop", applicationId);
      break;
    }
  }
}
