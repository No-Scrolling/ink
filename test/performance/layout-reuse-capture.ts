import { resolve } from "node:path";
import { mkdir } from "node:fs/promises";
import { decode, compare, matches, sha256 } from "../visual/pixels";
const label = Bun.argv[2];
if (!["baseline", "candidate"].includes(label) || !Bun.argv[3])
  throw Error(
    "Usage: bun test/performance/layout-reuse-capture.ts baseline|candidate OUTPUT_DIRECTORY",
  );
const directory = resolve(Bun.argv[3]);
await mkdir(directory, { recursive: true });
const adb = `${process.env.ANDROID_HOME}/platform-tools/adb`;
async function command(args: string[]) {
  if (args[0] === "shell" && args[1] === "input") {
    await command([
      "shell",
      "am",
      "start",
      "-W",
      "-n",
      "dev.ink.layoutreuse/com.vandam.ink.MainActivity",
    ]);
  }
  const child = Bun.spawn([adb, "-s", "emulator-5554", ...args], {
    stdout: "pipe",
    stderr: "pipe",
  });
  const [bytes, stderr, code] = await Promise.all([
    new Response(child.stdout).bytes(),
    new Response(child.stderr).text(),
    child.exited,
  ]);
  if (code) throw Error(stderr);
  return bytes;
}
if (
  new TextDecoder().decode(await command(["emu", "avd", "name"])).split(/\r?\n/)[0] !==
  "Light_Phone_III"
)
  throw Error("Expected the Light_Phone_III emulator");
const captures: { name: string; width: number; height: number; hash: string; stable: number }[] =
  [];
async function capture(name: string) {
  let previous = "",
    stable = 0;
  for (let attempt = 0; attempt < 30; attempt++) {
    const activities = new TextDecoder().decode(
      await command(["shell", "dumpsys", "activity", "activities"]),
    );
    if (
      !activities
        .split(/\r?\n/)
        .some(
          (line) =>
            /(?:mResumedActivity|topResumedActivity)/.test(line) &&
            line.includes("dev.ink.layoutreuse/"),
        )
    ) {
      await command([
        "shell",
        "am",
        "start",
        "-W",
        "-n",
        "dev.ink.layoutreuse/com.vandam.ink.MainActivity",
      ]);
      previous = "";
      stable = 0;
      continue;
    }
    const path = resolve(directory, `${label}-${name}.png`);
    await Bun.write(path, await command(["exec-out", "screencap", "-p"]));
    const after = new TextDecoder().decode(
      await command(["shell", "dumpsys", "activity", "activities"]),
    );
    if (
      !after
        .split(/\r?\n/)
        .some(
          (line) =>
            /(?:mResumedActivity|topResumedActivity)/.test(line) &&
            line.includes("dev.ink.layoutreuse/"),
        )
    ) {
      previous = "";
      stable = 0;
      continue;
    }
    const image = decode(await Bun.file(path).bytes());
    if (image.width !== 1080 || image.height !== 1240)
      throw Error("Expected an unscaled 1080 × 1240 frame");
    const hash = sha256(image.data);
    stable = hash === previous ? stable + 1 : 1;
    previous = hash;
    if (stable >= 3) {
      if (captures.at(-1)?.hash === hash) throw Error(`State did not change: ${name}`);
      captures.push({ name, width: image.width, height: image.height, hash, stable });
      console.log(`${label} ${name}: ${hash}`);
      return;
    }
    await Bun.sleep(250);
  }
  throw Error(`unstable ${name}`);
}
await command([
  "shell",
  "am",
  "start",
  "-W",
  "-S",
  "-n",
  "dev.ink.layoutreuse/com.vandam.ink.MainActivity",
]);
await capture("counter-0");
await command(["shell", "input", "tap", "540", "680"]);
await capture("counter-1");
await command(["shell", "input", "tap", "540", "825"]);
await capture("list-start");
await command(["shell", "input", "swipe", "994", "150", "994", "1230", "1500"]);
await capture("list-scrolled");
await command(["shell", "input", "tap", "200", "185"]);
await capture("detail");
await command(["shell", "input", "keyevent", "KEYCODE_BACK"]);
await capture("list-return");
await command(["shell", "input", "keyevent", "KEYCODE_BACK"]);
await capture("counter-return");
await Bun.write(resolve(directory, `${label}-captures.json`), JSON.stringify(captures, null, 2));
if (label === "candidate") {
  const results = [];
  for (const { name } of captures) {
    const actual = decode(await Bun.file(resolve(directory, `candidate-${name}.png`)).bytes());
    const baseline = decode(await Bun.file(resolve(directory, `baseline-${name}.png`)).bytes());
    results.push({
      state: name,
      ...compare(actual, baseline, {
        name: "whole screen",
        x: 0,
        y: 0,
        width: 1080,
        height: 1240,
        meaning: "Identical production UI after the same input sequence",
      }),
    });
  }
  await Bun.write(resolve(directory, "emulator-pixels.json"), JSON.stringify(results, null, 2));
  console.log(results);
  if (results.some((x) => !matches(x)))
    throw Error("pixel mismatch outside the existing visual-suite shading tolerance");
}
