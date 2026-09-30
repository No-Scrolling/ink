import { deepStrictEqual, equal, match } from "node:assert/strict";
import { mkdir, rm, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { resolve } from "node:path";
import "./prepare-app";

const args = process.argv.slice(2);
if (args.length !== 2 || args[0] !== "--serial" || !/^[\w.:-]+$/.test(args[1])) {
  throw new Error("Usage: bun test/android/app.ts --serial SERIAL");
}
const serial = args[1];
const root = resolve(import.meta.dir, "../..");
const fixture = resolve(root, "test/fixtures/android-app/contracts");
const packageName = "com.vandam.ink.sdkcontracts";
const output = resolve(root, ".test-output/android-app", new Date().toISOString().replaceAll(":", "-") + `-${process.pid}`);
await mkdir(output, { recursive: true });
const lock = resolve(root, ".test-output", `android-${serial}.lock`);
const devicePort = 18765;
let phase = 1;
const requests: { path: string; method: string; bytes: number[]; contentType: string | null; fixtureHeader: string | null }[] = [];
const upload = [0, 255, 99, 97, 102, 195, 169];
const streamed = [0, 255, 13, 10, 226, 130, 172];
const server = Bun.serve({
  hostname: "127.0.0.1", port: 0,
  async fetch(request) {
    const path = new URL(request.url).pathname;
    requests.push({ path, method: request.method, bytes: Array.from(new Uint8Array(await request.arrayBuffer())),
      contentType: request.headers.get("content-type"), fixtureHeader: request.headers.get("x-ink-fixture") });
    if (path === "/phase") return new Response(String(phase));
    if (path === "/echo" || path === "/retained") return new Response(new Uint8Array(upload), { status: 201, headers: { "x-ink-reply": "received" } });
    if (path === "/redirect307") return new Response(null, { status: 307, headers: { location: "/retained" } });
    if (path === "/redirect303") return new Response(null, { status: 303, headers: { location: "/rewritten" } });
    if (path === "/rewritten") return new Response("GET without upload");
    if (path === "/stream") return new Response(new ReadableStream({
      start(controller) {
        controller.enqueue(new Uint8Array([0, 255, 13]));
        controller.enqueue(new Uint8Array([10, 226, 130, 172]));
        controller.close();
      },
    }));
    if (path === "/held") return new Response(new ReadableStream({ start(controller) { controller.enqueue(new Uint8Array([7])); } }));
    if (path === "/healthy") return new Response("ready after abort");
    return new Response("Unexpected fixture request", { status: 404 });
  },
});

async function command(argv: string[], name: string, timeoutMs = 30_000, env = process.env) {
  const child = Bun.spawn(argv, { cwd: root, env, detached: true, stdout: "pipe", stderr: "pipe" });
  const timer = setTimeout(() => {
    try { process.kill(-child.pid, "SIGKILL"); }
    catch (error) {
      if (!(error instanceof Error && "code" in error && error.code === "ESRCH")) console.error(error);
    }
  }, timeoutMs);
  try {
    const [stdout, stderr, code] = await Promise.all([new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited]);
    await writeFile(resolve(output, `${name}.log`), stdout + stderr);
    if (code !== 0) throw new Error(`${name} failed (${code}); see ${output}/${name}.log`);
    return stdout;
  } finally { clearTimeout(timer); }
}
const adb = (args: string[], name: string) => command(["adb", "-s", serial, ...args], name);
async function observe(expectedPhase: number): Promise<Record<string, unknown>> {
  await adb(["shell", "am", "start", "-W", "-S", "-n", `${packageName}/com.vandam.ink.MainActivity`], `launch-${expectedPhase}`);
  const deadline = Date.now() + 90_000;
  while (Date.now() < deadline) {
    const pid = (await adb(["shell", "pidof", packageName], `pid-${expectedPhase}`)).trim();
    if (pid) {
      const log = await adb(["logcat", "--pid", pid, "-d", "-v", "raw", "-s", "Ink:I"], `observations-${expectedPhase}`);
      const marker = log.split("\n").find(line => line.startsWith("INK_CONTRACT_RESULT "));
      if (marker) {
        const result = JSON.parse(marker.slice("INK_CONTRACT_RESULT ".length));
        await writeFile(resolve(output, `phase-${expectedPhase}.json`), JSON.stringify(result, null, 2) + "\n");
        equal(result.version, 1);
        equal(result.phase, expectedPhase);
        if (result.error) throw new Error(`Fixture failed: ${result.error}`);
        return result.details;
      }
    }
    await Bun.sleep(250);
  }
  throw new Error(`SDK phase ${expectedPhase} did not finish; see ${output}`);
}

let locked = false, installed = false, reversed = false;
const failures: string[] = [];
const errors: string[] = [];
let passed = 0;
function contract(name: string, verify: () => void) {
  try { verify(); passed++; console.log(`PASS ${name}`); }
  catch (error) { failures.push(name); console.error(`FAIL ${name}: ${error}`); }
}
try {
  if ((await adb(["shell", "getprop", "sys.boot_completed"], "boot")).trim() !== "1") throw new Error("Device has not completed boot.");
  await mkdir(lock); locked = true;
  const packages = await adb(["shell", "pm", "list", "packages", packageName], "existing-package");
  if (packages.split("\n").some(line => line.trim() === `package:${packageName}`)) throw new Error(`Refusing to replace existing fixture ${packageName}.`);
  const reverse = await adb(["reverse", "--list"], "existing-reverse");
  if (reverse.split("\n").some(line => line.split(/\s+/)[1] === `tcp:${devicePort}`)) throw new Error(`Device port ${devicePort} already has a reverse mapping.`);
  let javaHome = process.env.JAVA_HOME;
  if (!javaHome && process.platform === "darwin") javaHome = (await command(["/usr/libexec/java_home", "-v", "17"], "java-home")).trim();
  const sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT ?? resolve(homedir(), "Library/Android/sdk");
  await command(["cargo", "run", "--profile", "ink-dev", "-p", "ink-cli", "--", "-C", fixture, "build", "--debug"], "build", 600_000,
    { ...process.env, JAVA_HOME: javaHome, ANDROID_HOME: sdk, INK_SDK_ROOT: root });
  await adb(["reverse", `tcp:${devicePort}`, `tcp:${server.port}`], "reverse"); reversed = true;
  await adb(["install", resolve(fixture, ".ink/build/android/outputs/apk/debug/app-debug.apk")], "install"); installed = true;
  const first = await observe(1);
  phase = 2;
  await adb(["shell", "am", "force-stop", packageName], "stop-between-phases");
  const reopened = await observe(2);
  contract("store migration decodes and commits the upgraded value", () => {
    equal(first.migrated, 42); equal(first.migrations, 1); equal(first.committedMigration, 42);
  });
  contract("competing SDK updates preserve both increments", () => equal(first.concurrent, 44));
  contract("sibling SDK subscriptions see committed data", () => equal(first.subscribed, 44));
  contract("older SDK store cannot read a newer schema", () => match(String(first.downgrade), /uses a newer version/));
  contract("stored migration and updates survive process restart", () => { equal(reopened.persisted, 44); equal(reopened.migrations, 0); });
  contract("public fetch transfers exact binary request and response bytes", () => {
    deepStrictEqual(first.echo, { status: 201, header: "received", bytes: upload });
    const echo = requests.find(request => request.path === "/echo");
    equal(echo?.method, "POST"); equal(echo?.fixtureHeader, "binary"); deepStrictEqual(echo?.bytes, upload);
  });
  contract("307 redirect replays the upload and preserves POST", () => {
    deepStrictEqual(first.redirect307, { redirected: true, url: `http://127.0.0.1:${devicePort}/retained`, bytes: upload });
    const initial = requests.find(request => request.path === "/redirect307");
    equal(initial?.method, "POST"); deepStrictEqual(initial?.bytes, upload);
    const retained = requests.find(request => request.path === "/retained");
    equal(retained?.method, "POST"); deepStrictEqual(retained?.bytes, upload);
  });
  contract("303 redirect rewrites POST and removes body headers", () => {
    deepStrictEqual(first.redirect303, { redirected: true, body: "GET without upload" });
    const initial = requests.find(request => request.path === "/redirect303");
    equal(initial?.method, "POST"); deepStrictEqual(initial?.bytes, [100, 105, 115, 99, 97, 114, 100, 32, 109, 101]);
    match(initial?.contentType ?? "", /^text\/plain;\s*charset=utf-8$/i);
    const rewritten = requests.find(request => request.path === "/rewritten");
    equal(rewritten?.method, "GET"); deepStrictEqual(rewritten?.bytes, []); equal(rewritten?.contentType, null);
  });
  contract("response reader preserves binary bytes and reaches EOF", () => deepStrictEqual(first.stream, { bytes: streamed, bodyUsed: true }));
  contract("abort rejects body reads and later requests still work", () => {
    deepStrictEqual(first.beforeAbort, [7]); equal(first.abort, "fixture-abort"); equal(first.recovered, "ready after abort");
  });
  contract("pre-aborted public fetch never reaches the server", () => {
    equal(first.preAbort, "before-send"); equal(requests.filter(request => request.path === "/must-not-arrive").length, 0);
  });
  console.log(`${passed} pass\n${failures.length} fail`);
  if (failures.length) process.exitCode = 1;
} catch (error) {
  errors.push(String(error)); console.error(error); process.exitCode = 1;
} finally {
  server.stop(true);
  if (installed) try { await adb(["uninstall", packageName], "uninstall"); } catch (error) { errors.push(String(error)); console.error(error); process.exitCode = 1; }
  if (reversed) try { await adb(["reverse", "--remove", `tcp:${devicePort}`], "remove-reverse"); } catch (error) { errors.push(String(error)); console.error(error); process.exitCode = 1; }
  if (locked) await rm(lock, { recursive: true, force: true });
  await writeFile(resolve(output, "requests.json"), JSON.stringify(requests, null, 2) + "\n");
  await writeFile(resolve(output, "result.json"), JSON.stringify({ passed, failures, errors }, null, 2) + "\n");
}
