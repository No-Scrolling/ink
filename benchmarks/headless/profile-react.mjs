import { resolve, dirname } from "node:path";
import { mkdir, copyFile } from "node:fs/promises";

function replaceOnce(source, before, after) {
  if (source.split(before).length !== 2) throw new Error(`Profiling hook changed: ${before.slice(0, 100)}`);
  return source.replace(before, () => after);
}

export function instrument(source, path) {
  if (path.endsWith("/react-reconciler.production.js")) {
    source = replaceOnce(source, '"use strict";', '"use strict";\nconst inkProfile = globalThis.__inkReactProfile;');
    for (const target of ["nextRenderLanes", "children"]) {
      source = replaceOnce(source, `${target} = Component(props, secondArg);`, `{
        const started = performance.now();
        try { ${target} = Component(props, secondArg); }
        finally { inkProfile.componentMs += performance.now() - started; inkProfile.componentCalls++; }
      }`);
    }
    for (const [name, condition] of [["workLoopSync", "null !== workInProgress"],
      ["workLoopConcurrentByScheduler", "null !== workInProgress && !shouldYield()"]]) {
      const loop = name === "workLoopSync"
        ? `for (; ${condition}; ) performUnitOfWork(workInProgress);`
        : `for (; ${condition}; )\n      performUnitOfWork(workInProgress);`;
      source = replaceOnce(source, `function ${name}() {\n    ${loop}\n  }`,
        `function ${name}() {
          const started = performance.now();
          try { for (; ${condition}; ) performUnitOfWork(workInProgress); }
          finally { inkProfile.workLoopMs += performance.now() - started; }
        }`);
    }
    if (process.env.INK_REACT_PROFILE === "jsx") {
      source = replaceOnce(source, "function performUnitOfWork(unitOfWork) {", "function performUnitOfWork(unitOfWork) {\n    inkProfile.workUnits++;");
    }
  }
  if (process.env.INK_REACT_PROFILE === "jsx" && path.endsWith("/react-jsx-runtime.production.js")) {
    source = replaceOnce(source, '"use strict";', '"use strict";\nconst inkProfile = globalThis.__inkReactProfile;');
    source = replaceOnce(source, "function jsxProd(type, config, maybeKey) {", `function jsxProd(type, config, maybeKey) {
      const sampled = (inkProfile.jsxCalls++ & 31) === inkProfile.phase;
      const started = sampled ? performance.now() : 0;`);
    source = replaceOnce(source, "  return {\n    $$typeof: REACT_ELEMENT_TYPE", "  const element = {\n    $$typeof: REACT_ELEMENT_TYPE");
    source = replaceOnce(source, "\n  };\n}\nexports.Fragment", `
  };
  if (sampled) { inkProfile.jsxMeasuredMs += performance.now() - started; inkProfile.jsxSamples++; }
  return element;
}
exports.Fragment`);
  }
  if (path.endsWith("/packages/ink/src/renderer.ts")) {
    source += "\nconst inkProfile = globalThis.__inkReactProfile;\n";
    source = replaceOnce(source, "export function dispatchEvent(id: number, name: string, args: unknown[]) {", "export function dispatchEvent(id: number, name: string, args: unknown[]) {\n  inkProfile.begin();");
    source = replaceOnce(source, "prepareForCommit: () => null,", "prepareForCommit: () => { inkProfile.renderEnd = performance.now(); return null; },");
    source = replaceOnce(source, "resetAfterCommit: () => {", "resetAfterCommit: () => {\n    inkProfile.commitEnd = performance.now();");
    source = replaceOnce(source, 'if (committed.length) __inkCommit(committed);', `if (committed.length) {
        const transferStart = performance.now();
        __inkCommit(committed);
        const transferEnd = performance.now();
        const profile = inkProfile.finish(transferStart, transferEnd);
        __inkPost(JSON.stringify({ type: "react-profile", reactProfile: profile ?? null }));
      }`);
    if (process.env.INK_REACT_PROFILE === "host") {
      source = replaceOnce(source, "function update(instance: Instance, type: string, _oldProps: Props, props: Props) {",
        "function unprofiledUpdate(instance: Instance, type: string, _oldProps: Props, props: Props) {");
      source += `\nfunction update(instance, type, previous, props) {
        if ((inkProfile.hostUpdateCalls++ & 31) !== inkProfile.phase) return unprofiledUpdate(instance, type, previous, props);
        const started = performance.now();
        try { return unprofiledUpdate(instance, type, previous, props); }
        finally { inkProfile.hostUpdateMeasuredMs += performance.now() - started; inkProfile.hostUpdateSamples++; }
      }\n`;
    }
  }
  return source;
}

const prelude = `globalThis.__inkReactProfile = {
  sequence: 0, phase: 0, active: false, componentMs: 0, componentCalls: 0,
  jsxMeasuredMs: 0, jsxSamples: 0, jsxCalls: 0, workLoopMs: 0, workUnits: 0,
  hostUpdateCalls: 0, hostUpdateMeasuredMs: 0, hostUpdateSamples: 0,
  begin() {
    this.phase = this.sequence++ & 31;
    this.componentMs = this.componentCalls = this.jsxMeasuredMs = this.jsxSamples = this.jsxCalls = 0;
    this.workLoopMs = this.workUnits = 0;
    this.hostUpdateCalls = this.hostUpdateMeasuredMs = this.hostUpdateSamples = 0;
    this.active = true;
    this.started = performance.now();
  },
  finish(transferStart, transferEnd) {
    if (!this.active) return;
    this.active = false;
    return {
      renderMs: this.renderEnd - this.started,
      workLoopMs: this.workLoopMs, workUnits: this.workUnits,
      componentMs: this.componentMs, componentCalls: this.componentCalls,
      commitMs: this.commitEnd - this.renderEnd,
      effectsMs: transferStart - this.commitEnd,
      nativeTransferMs: transferEnd - transferStart,
      jsxMeasuredMs: this.jsxMeasuredMs, jsxSamples: this.jsxSamples, jsxCalls: this.jsxCalls,
      clockPairMs: this.clockPairMs,
      hostUpdateCalls: this.hostUpdateCalls, hostUpdateMeasuredMs: this.hostUpdateMeasuredMs, hostUpdateSamples: this.hostUpdateSamples,
    };
  },
};
{
  const samples = [];
  for (let i = 0; i < 256; i++) {
    const started = performance.now();
    samples.push(performance.now() - started);
  }
  samples.sort((a, b) => a - b);
  globalThis.__inkReactProfile.clockPairMs = samples[128];
}\n`;

if (import.meta.main) {
  const [app, assets, mode] = Bun.argv.slice(2);
  if (!app || !assets || !["coarse", "jsx", "host"].includes(mode)) {
    throw new Error("Usage: profile-react.mjs APP ASSETS coarse|jsx|host (prepare assets first)");
  }
  const build = resolve(dirname(assets), "profile-build");
  const compiler = resolve(import.meta.dir, "../../crates/ink-compiler/src");
  await mkdir(build, { recursive: true });
  for (const name of ["icon-usage.js", "file-routes.js", "native-lists.js"]) {
    await copyFile(resolve(compiler, name), resolve(build, name));
  }
  let source = await Bun.file(resolve(compiler, "bundle-javascript.js")).text();
  source = `import { instrument } from ${JSON.stringify(import.meta.path)};\n` + source;
  source = replaceOnce(source, "      return { contents, loader: extension", "      contents = instrument(contents, path);\n      return { contents, loader: extension");
  const script = resolve(build, "build.js");
  await Bun.write(script, source);
  const output = resolve(assets, "app.js");
  const buildProcess = Bun.spawn([Bun.argv[0], script, resolve(app), resolve(app, ".ink/entry.tsx"), output, "release"], {
    env: { ...Bun.env, INK_REACT_PROFILE: mode }, stdout: "inherit", stderr: "inherit",
  });
  if (await buildProcess.exited !== 0) throw new Error("Could not build diagnostic bundle");
  await Bun.write(output, prelude + await Bun.file(output).text());
}
