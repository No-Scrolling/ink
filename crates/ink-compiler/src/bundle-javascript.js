import { dirname, resolve, relative, extname } from "node:path";
import { realpath } from "node:fs/promises";
const [root, entry, output, profile = "release"] = Bun.argv.slice(2);
const development = profile === "development";
const splitWeb = !development && process.env.INK_SPLIT_WEB !== "0" && output.endsWith("/app.js");
const inputs = new Set();
const capabilities = new Set();
const assets = new Map();
const icons = new Map();
const declarations = new Map();
const shared = new Map();
const components = new Set();
const persistentModules = new Map();
const sourceMaps = new Map();
const bundleDirectory = await realpath(process.cwd());
const ts = await import(Bun.resolveSync("typescript", root));
ts.getParsedCommandLineOfConfigFile(resolve(root, "tsconfig.json"), {}, {
  ...ts.sys,
  readFile(path) {
    const contents = ts.sys.readFile(path);
    if (contents !== undefined) inputs.add(resolve(path));
    return contents;
  },
  onUnRecoverableConfigFileDiagnostic(diagnostic) {
    throw new Error(ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n"));
  },
});
for (let directory = root; ; directory = dirname(directory)) {
  for (const name of ["bun.lock", "bun.lockb", "package-lock.json", "pnpm-lock.yaml", "yarn.lock"]) {
    const path = resolve(directory, name);
    if (await Bun.file(path).exists()) inputs.add(await realpath(path));
  }
  if (dirname(directory) === directory) break;
}
const frameworkDirectory = dirname(Bun.resolveSync("ink", root));
const remapping = development ? (await import(Bun.resolveSync("@jridgewell/remapping", frameworkDirectory))).default : null;
const babel = development ? await import(Bun.resolveSync("@babel/core", frameworkDirectory)) : null;
const refreshPlugin = development ? (await import(Bun.resolveSync("react-refresh/babel", frameworkDirectory))).default : null;
const commonjsPlugin = development ? (await import(Bun.resolveSync("@babel/plugin-transform-modules-commonjs", frameworkDirectory))).default : null;
const typescriptPlugin = development ? (await import(Bun.resolveSync("@babel/plugin-transform-typescript", frameworkDirectory))).default : null;
async function inspect(path) {
  path = await realpath(path);
  inputs.add(path);
  let directory = dirname(path);
  while (true) {
    const metadataPath = resolve(directory, "package.json");
    if (await Bun.file(metadataPath).exists()) {
      inputs.add(metadataPath);
      if (!declarations.has(directory)) {
        const pkg = await Bun.file(metadataPath).json();
        const declarationPath = resolve(directory, "ink-native.json");
        let declaration;
        if (await Bun.file(declarationPath).exists()) {
          inputs.add(declarationPath);
          declaration = await Bun.file(declarationPath).json();
          if (declaration.version !== 1 || !declaration.modules || typeof declaration.modules !== "object" || Array.isArray(declaration.modules) || Object.values(declaration.modules).some(names => !Array.isArray(names) || names.some(name => typeof name !== "string"))) throw new Error(`Unsupported native requirements declaration: ${declarationPath}`);
        } else if (pkg.name === "ink" || pkg.name?.startsWith("@ink/")) {
          throw new Error(`Package ${pkg.name} is missing ink-native.json; reinstall a compatible Ink package or declare its native requirements.`);
        }
        declarations.set(directory, { pkg, declaration });
      }
      const { pkg, declaration } = declarations.get(directory);
      if (declaration) {
        const module = relative(directory, path).replaceAll("\\", "/");
        for (const name of [...(declaration.modules["*"] ?? []), ...(declaration.modules[module] ?? [])]) capabilities.add(name);
      }
      return pkg.name;
    }
    const parent = dirname(directory);
    if (parent === directory) return;
    directory = parent;
  }
}
async function addAsset(path) {
  inputs.add(await realpath(path));
  const bytes = await Bun.file(path).bytes();
  const name = `${new Bun.CryptoHasher("sha256").update(bytes).digest("hex")}${extname(path).toLowerCase()}`;
  assets.set(name, path);
  return `${extname(path).toLowerCase() === ".mp3" ? "asset:///" : "asset://"}ink-assets/${name}`;
}
function buildOptions(bootstrap = false) { return {
  entrypoints: [entry], target: "browser", format: development && !bootstrap ? "cjs" : "iife", minify: !development,
  sourcemap: development ? "external" : "none",
  define: { "process.env.NODE_ENV": JSON.stringify(development ? "development" : "production") },
  plugins: [{ name: "ink-resolved-inputs", setup(build) {
    build.onLoad({ filter: /\.ink-icons$/ }, async ({ path }) => {
      await inspect(path);
      const descriptor = await Bun.file(path).json();
      if (descriptor.version !== 1 || !Array.isArray(descriptor.names) || !Number.isInteger(descriptor.resolution) || descriptor.resolution < 16 || descriptor.resolution > 128) throw new Error(`Invalid icon collection ${path}: version 1, names and resolution 16–128 required`);
      const references = {};
      for (const name of descriptor.names) {
        if (typeof name !== "string") throw new Error(`Invalid icon name in ${path}`);
        references[name] = name;
        for (const filled of [false, true]) icons.set(`${name}:${filled}`, { name, filled, size: Math.max(icons.get(`${name}:${filled}`)?.size ?? 0, descriptor.resolution) });
      }
      return { contents: `export default Object.freeze(${JSON.stringify(references)});`, loader: "js" };
    });
    build.onLoad({ filter: /\.(?:png|jpe?g|webp|mp3)$/i }, async ({ path }) => ({ contents: `export default ${JSON.stringify(await addAsset(path))}`, loader: "js" }));
    build.onResolve({ filter: /.*/ }, async ({ path, importer }) => {
      const resolved = await realpath(Bun.resolveSync(path, /^(?:react|ink)(?:\/|$)/.test(path) ? root : importer ? dirname(importer) : root));
      const name = await inspect(resolved);
      if (development && !bootstrap && importer && (name === "ink" || name === "react" || name?.startsWith("@ink/") || resolved.includes("/node_modules/"))) {
        shared.set(path, resolved);
        return { path: resolved, external: true };
      }
      return { path: resolved };
    });
    build.onLoad({ filter: /\.[cm]?[jt]sx?$/ }, async ({ path }) => {
      await inspect(path);
      const extension = extname(path).slice(1);
      let contents = await Bun.file(path).text();
      if (splitWeb && path === resolve(frameworkDirectory, "web.ts")) {
        contents = 'import * as native from "./native";\nlet web;\nfunction loadWeb() { if (!web) { __inkLoadWeb(); web = globalThis.__inkWebFactory(native); delete globalThis.__inkWebFactory; } return web; }\n' + contents.replace('require("./web-globals")[name]', 'loadWeb()[name]');
      }
      if (development && !bootstrap && !path.includes("/node_modules/")) {
        const original = contents;
        let transformed = await babel.transformAsync(contents, {
          filename: path, babelrc: false, configFile: false, sourceMaps: true, ast: true,
          plugins: [[typescriptPlugin, { isTSX: extension === "tsx", allExtensions: true }], [refreshPlugin, { skipEnvCheck: true }]],
          parserOpts: { plugins: ["tsx", "jsx"].includes(extension) ? ["jsx"] : [] },
        });
        let prefix = 'const $RefreshReg$ = (type, id) => globalThis.__inkFramework.refresh.register(type, ' + JSON.stringify(path + ' ') + ' + id);\nconst $RefreshSig$ = globalThis.__inkFramework.refresh.createSignatureFunctionForTransform;\n';
        let suffix = "";
        const mixedExports = transformed.ast.program.body.some(node => {
          if (node.type === "ExportAllDeclaration") return true;
          if (node.type !== "ExportNamedDeclaration") return false;
          if (node.specifiers.length) return true;
          const declaration = node.declaration;
          if (declaration?.type === "FunctionDeclaration") return !/^[A-Z]/.test(declaration.id?.name ?? "");
          if (declaration?.type === "VariableDeclaration") return declaration.declarations.some(item => item.id.type !== "Identifier" || !/^[A-Z]/.test(item.id.name));
          return !!declaration;
        });
        if (transformed.code.includes("$RefreshReg$(") && !mixedExports) {
          components.add(path);
        } else if (!path.includes("/.ink/")) {
          persistentModules.set(path, new Bun.CryptoHasher("sha256").update(original).digest("hex"));
          transformed = await babel.transformAsync(transformed.code, { filename: path, babelrc: false, configFile: false, sourceMaps: true, inputSourceMap: transformed.map, plugins: [commonjsPlugin], parserOpts: { plugins: ["jsx"] } });
          const key = JSON.stringify(path);
          prefix = 'if (globalThis.__inkFramework.appModules[' + key + ']) { module.exports = globalThis.__inkFramework.appModules[' + key + ']; } else {\n' + prefix;
          suffix = '\nglobalThis.__inkFramework.appModules[' + key + '] = module.exports;\n}';
        }
        transformed.map.mappings = ";".repeat(prefix.split("\n").length - 1) + transformed.map.mappings;
        transformed.map.sources = [path];
        sourceMaps.set(path, transformed.map);
        contents = prefix + transformed.code + suffix;
      }
      return { contents, loader: extension === "tsx" || extension === "jsx" ? extension : ["ts", "mts", "cts"].includes(extension) ? "ts" : "js" };
    });
    build.onLoad({ filter: /\.json$/ }, async ({ path }) => {
      await inspect(path);
      return { contents: await Bun.file(path).text(), loader: "json" };
    });
  }}],
}; }
let result = await Bun.build(buildOptions());
if (splitWeb) {
  const web = await Bun.build({
    entrypoints: [resolve(frameworkDirectory, "web-globals.ts")],
    target: "browser", format: "cjs", minify: true,
    define: { "process.env.NODE_ENV": '"production"' },
    plugins: [{ name: "ink-web-shared-native", setup(build) {
      build.onResolve({ filter: /.*/ }, async ({ path, importer }) => {
        const resolved = await realpath(Bun.resolveSync(path, importer ? dirname(importer) : frameworkDirectory));
        await inspect(resolved);
        return resolved === resolve(frameworkDirectory, "native.ts")
          ? { path: resolved, namespace: "ink-native" } : { path: resolved };
      });
      build.onLoad({ filter: /.*/, namespace: "ink-native" }, () => ({
        contents: 'export const { NativeError, callNative, onNativeMessage } = inkNative;', loader: "js",
      }));
    } }],
  });
  if (!web.success) throw new AggregateError(web.logs, "Could not build optional web runtime");
  const path = resolve(dirname(output), "ink-web.js");
  await Bun.write(path, 'globalThis.__inkWebFactory = function(inkNative) { const module = {exports:{}}; const exports = module.exports;\n' + await web.outputs[0].text() + '\nreturn module.exports; };');
  assets.set("ink-web.js", path);
}
if (development && result.success && components.size) {
  const refreshEntry = resolve(dirname(output), "refresh-entry.js");
  await Bun.write(refreshEntry, [...components].sort().map(path => "import " + JSON.stringify(path) + ";").join("\n") + "\nimport " + JSON.stringify(entry) + ";\n");
  const options = buildOptions();
  options.entrypoints = [refreshEntry];
  result = await Bun.build(options);
}
if (!result.success) { for (const log of result.logs) console.error(log); process.exit(1); }
let devRuntimeHash;
if (development) {
  const bootstrapEntry = resolve(dirname(output), "framework-entry.js");
  const refreshPath = Bun.resolveSync("react-refresh/runtime", frameworkDirectory);
  let bootstrapSource = 'import Refresh from ' + JSON.stringify(refreshPath) + ';\nRefresh.injectIntoGlobalHook(globalThis);\nglobalThis.__inkFramework = {refresh: Refresh, modules: Object.create(null), appModules: Object.create(null)};\n';
  for (const path of [...shared.keys()].sort()) bootstrapSource += 'globalThis.__inkFramework.modules[' + JSON.stringify(path) + '] = require(' + JSON.stringify(shared.get(path)) + ');\n';
  await Bun.write(bootstrapEntry, bootstrapSource);
  const options = buildOptions(true);
  options.entrypoints = [bootstrapEntry];
  options.sourcemap = "none";
  const bootstrap = await Bun.build(options);
  if (!bootstrap.success) throw new AggregateError(bootstrap.logs, "Could not build development framework");
  const framework = await bootstrap.outputs[0].text();
  devRuntimeHash = new Bun.CryptoHasher("sha256").update(framework).digest("hex");
  const prefix = 'if (!globalThis.__inkFramework) {\n' + framework + '\n}\n(function(require, module, exports) {\n';
  const suffix = '\n})(id => { if (!(id in globalThis.__inkFramework.modules)) throw new Error("Development module missing: " + id); return globalThis.__inkFramework.modules[id]; }, {exports:{}}, {});\nglobalThis.__inkFramework.refresh.performReactRefresh();\n';
  const code = await result.outputs.find(item => item.kind !== "sourcemap").text();
  await Bun.write(output, prefix + code + suffix);
  const mapArtifact = result.outputs.find(item => item.kind === "sourcemap");
  if (mapArtifact) {
    const map = JSON.parse(await mapArtifact.text());
    map.mappings = ";".repeat(prefix.split("\n").length - 1) + map.mappings;
    const originalMap = remapping(map, (source, context) => context.depth === 1 ? sourceMaps.get(resolve(bundleDirectory, source)) : null);
    await Bun.write(output + ".map", JSON.stringify(originalMap));
  }
} else {
  for (const artifact of result.outputs) await Bun.write(artifact.kind === "sourcemap" ? output + ".map" : output, artifact);
}
await Bun.write(`${output}.metadata.json`, JSON.stringify({ devRuntimeHash, refreshCompatibilityHash: development ? new Bun.CryptoHasher("sha256").update(JSON.stringify([...persistentModules].sort())).digest("hex") : undefined, inputs: [...inputs].sort(), capabilities: [...capabilities].sort(), assets: [...assets].map(([name,path])=>({name,path})), icons: [...icons.values()] }));
