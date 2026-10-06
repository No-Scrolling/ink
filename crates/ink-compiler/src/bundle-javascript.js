import { dirname, resolve, relative, extname } from "node:path";
import { realpath } from "node:fs/promises";
import { parseEnv } from "node:util";
import { collectIconSizes } from "./icon-usage.js";
import { compileNativeLists } from "./native-lists.js";
const [root, entry, output, profile = "release", routing] = Bun.argv.slice(2);
const development = profile === "development";
if (routing === "routes") {
  const { generateFileRoutes } = await import("./file-routes.js");
  await generateFileRoutes(root, resolve(dirname(entry), "routes.tsx"));
}
const splitWeb = !development && process.env.INK_SPLIT_WEB !== "0" && output.endsWith("/app.js");
const inputs = new Set();
const environment = {};
for (const name of [".env", ".env.local"]) {
  const path = resolve(root, name);
  if (await Bun.file(path).exists()) {
    inputs.add(path);
    Object.assign(environment, parseEnv(await Bun.file(path).text()));
  }
}
Object.assign(environment, process.env);
const publicEnvironment = Object.fromEntries(Object.entries(environment).filter(([key]) => key.startsWith("INK_PUBLIC_")));
const capabilities = new Set();
const moduleCapabilities = new Map();
const inspectedNativeModules = new Set();
const assets = new Map();
const icons = new Map();
const svgIcons = new Map();
const svgReferences = new Map();
const iconSizes = new Map();
const inspectedIconModules = new Set();
const declarations = new Map();
const shared = new Map();
const components = new Set();
const persistentModules = new Map();
const sourceMaps = new Map();
const bundleDirectory = await realpath(process.cwd());
const ts = await import(Bun.resolveSync("typescript", root));
const iconSource = await Bun.file(Bun.resolveSync("ink/icons", root)).text();
const materialIcons = new Map(Array.from(
  iconSource.matchAll(/export const (\w+) = "((?:outlined|filled):[a-z0-9_]+)"/g),
  ([, name, reference]) => [name, reference],
));
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
const oxc = await import(Bun.resolveSync("oxc-transform-react", frameworkDirectory));
const parser = development ? await import(Bun.resolveSync("oxc-parser", frameworkDirectory)) : null;
let networkEntry;
const remapping = development ? (await import(Bun.resolveSync("@jridgewell/remapping", frameworkDirectory))).default : null;

const compiledModules = new Map();
const mixedExportsByPath = new Map();
const pendingConversions = new Map();
const convertedModules = new Map();
let discovering = development;
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
        const names = [...(declaration.modules["*"] ?? []), ...(declaration.modules[module] ?? [])];
        moduleCapabilities.set(path, [...new Set([...(moduleCapabilities.get(path) ?? []), ...names])]);
        if (development) for (const name of names) capabilities.add(name);
      }
      if (pkg.name === "@ink/network" && path === resolve(directory, "src/index.ts")) networkEntry = path;
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
  return `${/\.(?:mp3|mp4|mov|webm)$/i.test(path) ? "asset:///" : "asset://"}ink-assets/${name}`;
}
async function svgReference(path) {
  path = await realpath(path);
  if (svgReferences.has(path)) return svgReferences.get(path);
  inputs.add(path);
  const bytes = await Bun.file(path).bytes();
  const name = `svg_${new Bun.CryptoHasher("sha256").update(bytes).digest("hex")}`;
  const reference = `outlined:${name}`;
  svgIcons.set(name, path);
  svgReferences.set(path, reference);
  return reference;
}
function buildOptions(bootstrap = false) { return {
  entrypoints: [entry], target: "browser", format: development && !bootstrap ? "cjs" : "iife", minify: !development,
  sourcemap: development ? "external" : "none",
  metafile: true,
  define: { "process.env.NODE_ENV": JSON.stringify(development ? "development" : "production"), "import.meta.env": JSON.stringify(publicEnvironment) },
  plugins: [{ name: "ink-resolved-inputs", setup(build) {
    build.onLoad({ filter: /\.svg$/i }, async ({ path }) => {
      return { contents: `export default ${JSON.stringify(await svgReference(path))}`, loader: "js" };
    });
    build.onLoad({ filter: /\.(?:png|jpe?g|gif|webp|mp3|mp4|mov|webm|db)$/i }, async ({ path }) => ({ contents: `export default ${JSON.stringify(await addAsset(path))}`, loader: "js" }));
    build.onResolve({ filter: /.*/ }, async ({ path, importer }) => {
      const resolved = await realpath(Bun.resolveSync(path, /^(?:react|ink)(?:\/|$)/.test(path) ? root : importer ? dirname(importer) : root));
      const name = await inspect(resolved);
      if (development && !bootstrap && importer && (name === "ink" || name === "react" || name?.startsWith("@ink/") || resolved.includes("/node_modules/"))) {
        shared.set(path, resolved);
        shared.set(resolved, resolved);
        return { path: resolved, external: true };
      }
      return { path: resolved };
    });
    build.onLoad({ filter: /\.[cm]?[jt]sx?$/ }, async ({ path }) => {
      await inspect(path);
      const extension = extname(path).slice(1);
      let contents = await Bun.file(path).text();
      if (!inspectedNativeModules.has(path)) {
        inspectedNativeModules.add(path);
        const source = ts.createSourceFile(path, contents, ts.ScriptTarget.Latest, true);
        const requirements = new Set(moduleCapabilities.get(path) ?? []);
        const families = { Canvas: "ui-canvas", CanvasIcon: "ui-canvas", CanvasRectangle: "ui-canvas", CanvasText: "ui-canvas", NativeList: "ui-lists", PlayingScreen: "ui-playing", MessageContent: "ui-messages" };
        function visit(node) {
          if (ts.isCallExpression(node)) {
            const callee = node.expression;
            const name = ts.isPropertyAccessExpression(callee) ? callee.name.text : ts.isIdentifier(callee) ? callee.text : "";
            if (["node", "createElement", "jsx", "jsxs"].includes(name) && node.arguments[0]) {
              const kind = node.arguments[0];
              if (ts.isStringLiteral(kind)) {
                if (Object.hasOwn(families, kind.text)) requirements.add(families[kind.text]);
                if (kind.text === "TextInput") requirements.add("text-input");
                if (["Image", "Avatar", "RowContent", "PlayingScreen"].includes(kind.text)) {
                  requirements.add("image"); requirements.add("network");
                }
              } else if (!path.startsWith(frameworkDirectory + "/") && !path.includes("/.ink/")) {
                // Dynamic host descriptions need every optional family.
                for (const feature of Object.values(families)) requirements.add(feature);
                requirements.add("image"); requirements.add("network"); requirements.add("text-input");
              }
            }
          }
          ts.forEachChild(node, visit);
        }
        visit(source);
        moduleCapabilities.set(path, [...requirements]);
        if (development) for (const name of requirements) capabilities.add(name);
      }
      if (!inspectedIconModules.has(path) && (contents.includes("ink/icons") || contents.includes(".svg"))) {
        inspectedIconModules.add(path);
        const sizes = await collectIconSizes(ts, path, contents, materialIcons,
          module => svgReference(Bun.resolveSync(module, dirname(path))));
        for (const [reference, size] of sizes) iconSizes.set(reference, Math.max(iconSizes.get(reference) ?? 0, size));
      }
      let compilerMap;
      contents = compileNativeLists(ts, path, contents);
      const appSource = path.startsWith(root + "/") && !path.includes("/node_modules/") && !path.includes("/.ink/");
      const refresh = development && !bootstrap && !path.includes("/node_modules/");
      if (appSource || refresh) {
        const cacheKey = path + ":" + refresh;
        const compiled = compiledModules.get(cacheKey) ?? await oxc.transform(path, contents, {
          reactCompiler: appSource ? { target: "19" } : false,
          jsx: { runtime: "automatic", development, refresh }, sourcemap: development,
        });
        if (compiled.fatal || compiled.errors.length) throw new Error(`Oxc could not compile ${path}: ${JSON.stringify(compiled.errors)}`);
        compiledModules.set(cacheKey, compiled);
        contents = compiled.code;
        compilerMap = compiled.map;
      }
      if (!development && path === networkEntry) {
        contents = 'import * as native from "ink/native";\nlet web;\nfunction loadWeb() { if (!web) { ' + (splitWeb ? '__inkLoadWeb(); ' : '') + 'web = globalThis.__inkWebFactory(native); delete globalThis.__inkWebFactory; } return web; }\n' + contents.replace('require("./web-globals")[name]', 'loadWeb()[name]');
      }
      if (refresh) {
        let mixedExports = mixedExportsByPath.get(path);
        if (mixedExports === undefined) {
          const parsed = parser.parseSync(path, contents, { lang: "js" });
          if (parsed.errors.length) throw new Error(`Could not analyse ${path}: ${JSON.stringify(parsed.errors)}`);
          mixedExports = parsed.program.body.some(node => {
            if (node.type === "ExportAllDeclaration") return true;
            if (node.type !== "ExportNamedDeclaration") return false;
            if (node.specifiers.length) return true;
            const declaration = node.declaration;
            if (declaration?.type === "FunctionDeclaration") return !/^[A-Z]/.test(declaration.id?.name ?? "");
            if (declaration?.type === "VariableDeclaration") return declaration.declarations.some(item => item.id.type !== "Identifier" || !/^[A-Z]/.test(item.id.name));
            return !!declaration;
          });
          mixedExportsByPath.set(path, mixedExports);
        }
        let transformed = { code: contents, map: compilerMap };
        let prefix = 'const $RefreshReg$ = (type, id) => globalThis.__inkFramework.refresh.register(type, ' + JSON.stringify(path + ' ') + ' + id);\nconst $RefreshSig$ = globalThis.__inkFramework.refresh.createSignatureFunctionForTransform;\n';
        let suffix = "";
        if (transformed.code.includes("$RefreshReg$(") && !mixedExports) {
          components.add(path);
        } else if (!path.includes("/.ink/")) {
          persistentModules.set(path, new Bun.CryptoHasher("sha256").update(contents).digest("hex"));
          if (discovering) {
            pendingConversions.set(path, transformed);
            return { contents: transformed.code, loader: "js" };
          }
          transformed = convertedModules.get(path);
          if (!transformed) throw new Error("Missing prepared module: " + path);
          const key = JSON.stringify(path);
          prefix = 'if (globalThis.__inkFramework.appModules[' + key + ']) { module.exports = globalThis.__inkFramework.appModules[' + key + ']; } else {\n' + prefix;
          suffix = '\nglobalThis.__inkFramework.appModules[' + key + '] = module.exports;\n}';
        }
        transformed = { ...transformed, map: structuredClone(transformed.map) };
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
if (development && result.success) {
  // Bun builds cannot be nested inside onLoad. Convert discovered modules between passes.
  for (const [path, prepared] of pendingConversions) {
    const converted = await Bun.build({
      entrypoints: [path], files: { [path]: prepared.code }, target: "browser", format: "cjs",
      external: ["*"], sourcemap: "external", minify: false,
    });
    if (!converted.success) throw new AggregateError(converted.logs, `Could not convert ${path}`);
    const code = await converted.outputs.find(item => item.kind !== "sourcemap").text();
    const map = JSON.parse(await converted.outputs.find(item => item.kind === "sourcemap").text());
    convertedModules.set(path, { code, map: remapping([map, prepared.map], () => null) });
  }
  discovering = false;
  const options = buildOptions();
  if (components.size) {
    const refreshEntry = resolve(dirname(output), "refresh-entry.js");
    await Bun.write(refreshEntry, [...components].sort().map(path => "import " + JSON.stringify(path) + ";").join("\n") + "\nimport " + JSON.stringify(entry) + ";\n");
    options.entrypoints = [refreshEntry];
  }
  result = await Bun.build(options);
}
let inlineWeb = "";
if (!development && networkEntry) {
  const web = await Bun.build({
    entrypoints: [resolve(dirname(networkEntry), "web-globals.ts")],
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
        contents: 'export const { NativeError, callNative, callNativeBytes, onNativeMessage } = inkNative;', loader: "js",
      }));
    } }],
  });
  if (!web.success) throw new AggregateError(web.logs, "Could not build web runtime");
  const factory = 'globalThis.__inkWebFactory = function(inkNative) { const module = {exports:{}}; const exports = module.exports;\n' + await web.outputs[0].text() + '\nreturn module.exports; };\n';
  if (splitWeb) {
    const path = resolve(dirname(output), "ink-web.js");
    await Bun.write(path, factory);
    assets.set("ink-web.js", path);
  } else {
    inlineWeb = factory;
  }
  for (const name of moduleCapabilities.get(networkEntry) ?? []) capabilities.add(name);
}
if (!result.success) { for (const log of result.logs) console.error(log); process.exit(1); }
if (!development) {
  for (const bundle of Object.values(result.metafile.outputs)) {
    for (const [path, contribution] of Object.entries(bundle.inputs)) {
      if (contribution.bytesInOutput === 0) continue;
      for (const name of moduleCapabilities.get(resolve(bundleDirectory, path)) ?? []) capabilities.add(name);
    }
  }
}
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
let bundledCode = await Bun.file(output).text();
if (inlineWeb) {
  bundledCode = inlineWeb + bundledCode;
  await Bun.write(output, bundledCode);
}
if (development) {
  const usedIcons = await Bun.build({ ...buildOptions(true), minify: true, sourcemap: "none" });
  if (!usedIcons.success) throw new AggregateError(usedIcons.logs, "Could not determine application icons");
  bundledCode = await usedIcons.outputs[0].text();
}
// Read references after tree-shaking, including those passed through props or collections.
function collectIcons(node) {
  if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) {
    const reference = node.text;
    const match = /^(outlined|filled):([a-z0-9_]+)$/.exec(reference);
    if (match) icons.set(reference, { name: match[2], filled: match[1] === "filled", reference, size: iconSizes.get(reference) ?? 56, svg: svgIcons.get(match[2]) });
  }
  ts.forEachChild(node, collectIcons);
}
collectIcons(ts.createSourceFile(output, bundledCode, ts.ScriptTarget.Latest, false, ts.ScriptKind.JS));
await Bun.write(`${output}.metadata.json`, JSON.stringify({ devRuntimeHash, refreshCompatibilityHash: development ? new Bun.CryptoHasher("sha256").update(JSON.stringify([...persistentModules].sort())).digest("hex") : undefined, inputs: [...inputs].sort(), capabilities: [...capabilities].sort(), assets: [...assets].map(([name,path])=>({name,path})), icons: [...icons.values()] }));
