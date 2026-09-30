import { expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const repository = resolve(import.meta.dir, "../..");
const plainPage = 'import { Screen, Text } from "ink";\nexport default function Page() { return <Screen><Text>Compiler contract</Text></Screen>; }\n';

async function withProject(run: (root: string) => Promise<void>) {
  const output = resolve(repository, ".test-output");
  await mkdir(output, { recursive: true });
  const root = await mkdtemp(resolve(output, "compiler-contract-"));
  try {
    await mkdir(resolve(root, "app"));
    await mkdir(resolve(root, "node_modules/@ink"), { recursive: true });
    for (const [name, path] of [
      ["ink", resolve(repository, "packages/ink")],
      ["@ink/network", resolve(repository, "packages/network")],
      ["react", dirname(Bun.resolveSync("react/package.json", resolve(repository, "packages/ink")))],
    ] as const) await symlink(path, resolve(root, "node_modules", name));
    await writeFile(resolve(root, "package.json"), '{"name":"compiler-contract","private":true}\n');
    await writeFile(resolve(root, "tsconfig.json"), JSON.stringify({
      compilerOptions: { jsx: "react-jsx", module: "ESNext", moduleResolution: "bundler", noEmit: true, skipLibCheck: true, strict: true, target: "ES2022" },
      include: ["app/**/*.tsx"],
    }));
    await writeFile(resolve(root, "ink.toml"), 'name = "Compiler contract"\npackage = "dev.ink.compilercontract"\n');
    await writeFile(resolve(root, "app/index.tsx"), plainPage);
    await run(root);
  } finally { await rm(root, { recursive: true, force: true }); }
}

async function compile(root: string, splitWeb = true) {
  const child = Bun.spawn(["cargo", "run", "--profile", "ink-dev", "-p", "ink-compiler", "--", "compile", resolve(root, "ink.toml")], {
    cwd: repository, stdout: "pipe", stderr: "pipe",
    env: { ...process.env, INK_SPLIT_WEB: splitWeb ? "1" : "0" },
  });
  const [exitCode, stdout, stderr] = await Promise.all([
    child.exited, new Response(child.stdout).text(), new Response(child.stderr).text(),
  ]);
  return { exitCode, output: stdout + stderr };
}

async function manifest(root: string) {
  return JSON.parse(await readFile(resolve(root, ".ink/android/assets/ink-capabilities-v1.json"), "utf8")) as { version: number; capabilities: string[] };
}

test("explicit recording and detached-playback capabilities include their transitive requirements once", async () => {
  await withProject(async root => {
    await writeFile(resolve(root, "ink.toml"), 'name = "Compiler contract"\npackage = "dev.ink.compilercontract"\ncapabilities = ["audio-recording", "audio-detached", "audio-recording"]\n');
    expect(await compile(root)).toMatchObject({ exitCode: 0 });
    expect(await manifest(root)).toEqual({ version: 1, capabilities: [
      "audio", "audio-capture", "audio-detached", "audio-playback", "audio-recording", "external", "files", "microphone-permission", "network",
    ] });
  });
}, 60_000);

test("surviving package code keeps its declared native requirement and discarded code does not", async () => {
  await withProject(async root => {
    const dependency = resolve(root, "node_modules/contract-native");
    await mkdir(dependency);
    await writeFile(resolve(dependency, "package.json"), '{"name":"contract-native","exports":"./index.ts","sideEffects":false}\n');
    await writeFile(resolve(dependency, "index.ts"), 'export function label() { return "Encrypted notes"; }\n');
    await writeFile(resolve(dependency, "ink-native.json"), '{"version":1,"modules":{"index.ts":["crypto"]}}\n');
    await writeFile(resolve(root, "app/index.tsx"), 'import { label } from "contract-native";\nimport { Screen, Text } from "ink";\nexport default function Page() { return <Screen><Text>{label()}</Text></Screen>; }\n');
    expect(await compile(root)).toMatchObject({ exitCode: 0 });
    expect(await manifest(root)).toEqual({ version: 1, capabilities: ["crypto", "external"] });
    await writeFile(resolve(root, "app/index.tsx"), 'import { label } from "contract-native";\n' + plainPage);
    expect(await compile(root)).toMatchObject({ exitCode: 0 });
    expect(await manifest(root)).toEqual({ version: 1, capabilities: ["external"] });
  });
}, 60_000);

test.each([
  { declaration: { version: 2, modules: { "index.ts": ["crypto"] } }, message: "Unsupported native requirements declaration" },
  { declaration: { version: 1, modules: { "index.ts": [123] } }, message: "Unsupported native requirements declaration" },
  { declaration: { version: 1, modules: { "index.ts": ["invented-capability"] } }, message: "invented-capability" },
])("rejects invalid package native declarations: $declaration", async ({ declaration, message }) => {
  await withProject(async root => {
    const dependency = resolve(root, "node_modules/contract-native");
    await mkdir(dependency);
    await writeFile(resolve(dependency, "package.json"), '{"name":"contract-native","exports":"./index.ts"}\n');
    await writeFile(resolve(dependency, "index.ts"), 'console.log("Native dependency loaded");\n');
    await writeFile(resolve(dependency, "ink-native.json"), JSON.stringify(declaration));
    await writeFile(resolve(root, "app/index.tsx"), 'import "contract-native";\n' + plainPage);
    const result = await compile(root);
    expect(result.exitCode).not.toBe(0);
    expect(result.output).toContain(message);
    expect(await Bun.file(resolve(root, ".ink/android/assets/app.js")).exists()).toBe(false);
  });
}, 60_000);

test("unknown configured capabilities fail before publishing an application", async () => {
  await withProject(async root => {
    await writeFile(resolve(root, "ink.toml"), 'name = "Compiler contract"\npackage = "dev.ink.compilercontract"\ncapabilities = ["invented-capability"]\n');
    const result = await compile(root);
    expect(result.exitCode).not.toBe(0);
    expect(result.output).toContain("invented-capability");
    expect(await Bun.file(resolve(root, ".ink/android/assets/app.js")).exists()).toBe(false);
  });
}, 60_000);

test("a catalogue-absent capability produces an actionable configuration error rather than a compiler panic", async () => {
  await withProject(async root => {
    await writeFile(resolve(root, "ink.toml"), 'name = "Compiler contract"\npackage = "dev.ink.compilercontract"\ncapabilities = ["text-input-full"]\n');
    const result = await compile(root);
    expect(result.exitCode).not.toBe(0);
    expect(result.output).toContain("text-input-full");
    expect(result.output).not.toContain("panicked");
    expect(await Bun.file(resolve(root, ".ink/android/assets/app.js")).exists()).toBe(false);
  });
}, 60_000);

test("an Ink package missing its native requirements declaration cannot silently omit capabilities", async () => {
  await withProject(async root => {
    const dependency = resolve(root, "node_modules/@ink/contract-native");
    await mkdir(dependency);
    await writeFile(resolve(dependency, "package.json"), '{"name":"@ink/contract-native","exports":"./index.ts"}\n');
    await writeFile(resolve(dependency, "index.ts"), 'console.log("Native dependency loaded");\n');
    await writeFile(resolve(root, "app/index.tsx"), 'import "@ink/contract-native";\n' + plainPage);
    const result = await compile(root);
    expect(result.exitCode).not.toBe(0);
    expect(result.output).toContain("Package @ink/contract-native is missing ink-native.json");
    expect(await Bun.file(resolve(root, ".ink/android/assets/app.js")).exists()).toBe(false);
  });
}, 60_000);

test("a TypeScript error rejects an otherwise bundleable application before publishing assets", async () => {
  await withProject(async root => {
    await writeFile(resolve(root, "app/index.tsx"), 'const title: string = 123;\nimport { Screen, Text } from "ink";\nexport default function Page() { return <Screen><Text>{title}</Text></Screen>; }\n');
    const result = await compile(root);
    expect(result.exitCode).not.toBe(0);
    expect(result.output).toContain("TS2322");
    expect(await Bun.file(resolve(root, ".ink/android/assets/app.js")).exists()).toBe(false);
    expect(await Bun.file(resolve(root, ".ink/android/assets/ink-capabilities-v1.json")).exists()).toBe(false);
  });
}, 60_000);

test("changing split-web mode removes the old optional asset while retaining the network capability", async () => {
  await withProject(async root => {
    await writeFile(resolve(root, "app/index.tsx"), 'import "@ink/network";\n' + plainPage);
    expect(await compile(root)).toMatchObject({ exitCode: 0 });
    const asset = resolve(root, ".ink/android/assets/ink-assets/ink-web.js");
    expect(await Bun.file(asset).exists()).toBe(true);
    expect(await manifest(root)).toEqual({ version: 1, capabilities: ["external", "network"] });
    expect(await compile(root, false)).toMatchObject({ exitCode: 0 });
    expect(await Bun.file(asset).exists()).toBe(false);
    expect(await manifest(root)).toEqual({ version: 1, capabilities: ["external", "network"] });
    await writeFile(resolve(root, "app/index.tsx"), plainPage);
    expect(await compile(root)).toMatchObject({ exitCode: 0 });
    expect(await manifest(root)).toEqual({ version: 1, capabilities: ["external"] });
  });
}, 60_000);
