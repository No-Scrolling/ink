import { parseArgs } from "node:util";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const { values, positionals } = parseArgs({
  args: process.argv.slice(2), allowPositionals: true,
  options: {
    profile: { type: "string", default: "ink-dev" },
    filter: { type: "string" },
    serial: { type: "string" },
    help: { type: "boolean" },
  },
});
const hostSuites = ["unit", "compiler", "sdk", "runtime", "scene", "audio"];
const suites = positionals.length ? [...new Set(positionals)] : hostSuites;
if (values.help) {
  console.log("Usage: bun run test [unit|compiler|sdk|runtime|scene|audio|android|android-app|visual ...] [--profile ink-dev|release] [--filter NAME] [--serial SERIAL]");
  console.log("Default: host correctness suites. Android requires an explicit serial. Performance is not timed by this runner.");
  process.exit(0);
}
if (!["ink-dev", "release"].includes(values.profile!)) throw new Error("Profile must be ink-dev or release.");
for (const suite of suites) if (![...hostSuites, "android", "android-app", "visual"].includes(suite)) throw new Error(`Unknown suite: ${suite}`);
const deviceSuites = suites.filter(suite => ["android", "android-app", "visual"].includes(suite));
if (deviceSuites.length && !values.serial) throw new Error("Android tests require --serial SERIAL.");
if (values.serial && !deviceSuites.length) throw new Error("--serial requires an Android suite.");
if (values.filter && deviceSuites.length) throw new Error("Android contract filtering is not supported; omit --filter.");

const output = resolve(root, ".test-output", new Date().toISOString().replaceAll(":", "-") + `-${process.pid}`);
await mkdir(output, { recursive: true });
const git = (...args: string[]) => {
  const result = Bun.spawnSync(["git", ...args], { cwd: root, stdout: "pipe", stderr: "pipe" });
  if (result.exitCode !== 0) throw new Error(result.stderr.toString());
  return result.stdout.toString();
};
const files = git("ls-files", "--cached", "--others", "--exclude-standard", "-z").split("\0").filter(Boolean);
const sourceHashes: Record<string, string> = {};
for (const file of new Set(files)) {
  if (!existsSync(resolve(root, file))) continue;
  sourceHashes[file] = new Bun.CryptoHasher("sha256").update(await readFile(resolve(root, file))).digest("hex");
}
await writeFile(resolve(output, "source-hashes.json"), JSON.stringify(sourceHashes, null, 2) + "\n");
await writeFile(resolve(output, "tracked-changes.patch"), git("diff", "--binary", "HEAD"));
const report = {
  revision: git("rev-parse", "HEAD").trim(), dirty: git("status", "--porcelain").length > 0,
  bun: Bun.version, profile: values.profile, suites, filter: values.filter,
  serial: values.serial, startedAt: new Date().toISOString(),
  commands: [] as { name: string; argv: string[]; exitCode: number; passed: number; failed: number }[],
};
const failures: string[] = [];
console.log(`Test evidence: ${output}`);

async function run(name: string, argv: string[]) {
  console.log(`\n${name}`);
  const child = Bun.spawn(argv, { cwd: root, stdout: "pipe", stderr: "pipe" });
  const log = Bun.file(resolve(output, `${name}.log`)).writer();
  async function stream(input: ReadableStream<Uint8Array>, destination: typeof process.stdout) {
    for await (const chunk of input) { log.write(chunk); destination.write(chunk); }
  }
  const [code] = await Promise.all([child.exited, stream(child.stdout, process.stdout), stream(child.stderr, process.stderr)]);
  await log.end();
  const content = await readFile(resolve(output, `${name}.log`), "utf8");
  let passed = 0, failed = 0;
  for (const result of content.matchAll(/test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed/g)) {
    passed += Number(result[1]); failed += Number(result[2]);
  }
  for (const result of content.matchAll(/(?:^|\n)\s*(\d+) (pass|fail)\b/g)) {
    if (result[2] === "pass") passed += Number(result[1]); else failed += Number(result[1]);
  }
  const androidSummary = content.match(/^INSTRUMENTATION_RESULT: ink\.summary=(.+)$/m);
  if (androidSummary) {
    const summary = JSON.parse(androidSummary[1]);
    passed = summary.passed; failed = summary.failures.length;
  }
  let exitCode = code;
  if (hostSuites.includes(name) && passed + failed === 0) {
    console.error(`${name} executed no tests; refusing an empty success.`);
    exitCode = 1;
  }
  report.commands.push({ name, argv, exitCode, passed, failed });
  if (exitCode !== 0) failures.push(name);
  await writeFile(resolve(output, "result.json"), JSON.stringify({ ...report, failures }, null, 2) + "\n");
  return exitCode === 0;
}

const nativeSuites = suites.filter(suite => ["runtime", "scene", "audio"].includes(suite));
const fixtures = nativeSuites.filter(suite => suite !== "audio");
const prepared = !fixtures.length || await run("prepare", [process.execPath, "test/native/prepare.ts", ...fixtures, "--profile", values.profile!]);
for (const suite of suites) {
  if (suite === "unit") {
    await run(suite, ["cargo", "test", "--no-fail-fast", "--profile", values.profile!, "--workspace", "--exclude", "ink-test", ...(values.filter ? [values.filter] : [])]);
  } else if (["compiler", "sdk"].includes(suite)) {
    await run(suite, [process.execPath, "test", `./test/${suite}`, ...(values.filter ? ["--test-name-pattern", values.filter] : [])]);
  } else if (suite === "android") {
    await run(suite, [process.execPath, "test/android/run.ts", "--serial", values.serial!]);
  } else if (suite === "android-app") {
    await run(suite, [process.execPath, "test/android/app.ts", "--serial", values.serial!]);
  } else if (suite === "visual") {
    await run(suite, [process.execPath, "test/visual/run.ts", "--serial", values.serial!]);
  } else if (suite === "audio" || prepared) {
    const targets = suite === "runtime" ? ["runtime", "split_web"] : suite === "scene" ? ["scene", "navigation"] : [suite];
    await run(suite, ["cargo", "test", "--no-fail-fast", "--profile", values.profile!, "-p", "ink-test", ...targets.flatMap(target => ["--test", target]), ...(values.filter ? [values.filter] : [])]);
  }
}
await writeFile(resolve(output, "result.json"), JSON.stringify({ ...report, failures, completedAt: new Date().toISOString() }, null, 2) + "\n");
console.log(failures.length ? `\nFailed: ${failures.join(", ")}. Evidence: ${output}` : "\nAll selected suites passed.");
process.exitCode = failures.length ? 1 : 0;
