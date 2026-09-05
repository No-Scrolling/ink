import { resolve, extname } from "node:path";
const [root, entry, output] = Bun.argv.slice(2);
const ts = await import(Bun.resolveSync("typescript", root));
const icons = new Map();
const strings = new Set();
const dynamicIcons = new Map();
const components = new Set();
const packages = new Set();
const assets = new Map();
let remoteImages = false;
let detachedAudio = false;
let audioPlayback = false;
let audioCapture = false;
let photoCapture = false;
let codeScanner = false;
async function addAsset(path) {
  if (!/\.(?:png|jpe?g|webp|mp3)$/i.test(path)) throw new Error(`Unsupported asset format: ${path}`);
  const bytes = await Bun.file(path).bytes();
  const hash = new Bun.CryptoHasher("sha256").update(bytes).digest("hex");
  const name = `${hash}${extname(path).toLowerCase()}`;
  assets.set(name, path);
  return `${extname(path).toLowerCase() === ".mp3" ? "asset:///" : "asset://"}ink-assets/${name}`;
}
function addIcon(name, size, filled) {
  const key = `${name}:${filled}`;
  const previous = icons.get(key);
  if (!previous || previous.size < size) icons.set(key, { name, size, filled });
}
function iconNames(node) {
  if (!node) return undefined;
  if (ts.isStringLiteralLike(node)) return [node.text];
  if (ts.isParenthesizedExpression(node) || ts.isAsExpression(node) || ts.isSatisfiesExpression(node)) return iconNames(node.expression);
  if (ts.isConditionalExpression(node)) {
    const yes = iconNames(node.whenTrue);
    const no = iconNames(node.whenFalse);
    if (yes && no) return [...yes, ...no];
  }
  return undefined;
}
async function inspectModule(path, contents) {
  const replacements = [];
  const source = ts.createSourceFile(path, contents, ts.ScriptTarget.Latest, true);
  const aliases = new Map();
  const players = new Set();
  const imageImports = new Set();
  for (const statement of source.statements) {
    if (ts.isImportDeclaration(statement) && statement.moduleSpecifier.text === "@ink/camera" && !statement.importClause?.isTypeOnly) {
      const bindings = statement.importClause?.namedBindings;
      if (bindings && ts.isNamedImports(bindings)) {
        for (const binding of bindings.elements) {
          if (binding.isTypeOnly) continue;
          const name = binding.propertyName?.text ?? binding.name.text;
          if (name === "useCamera") photoCapture = true;
          if (name === "useCodeScanner") codeScanner = true;
        }
      } else if (bindings && ts.isNamespaceImport(bindings)) {
        photoCapture = true;
        codeScanner = true;
      }
    }
    if (ts.isImportDeclaration(statement) && statement.moduleSpecifier.text === "@ink/audio") {
      const bindings = statement.importClause?.namedBindings;
      if (!statement.importClause?.isTypeOnly && bindings && ts.isNamedImports(bindings)) {
        for (const binding of bindings.elements) {
          if (!binding.isTypeOnly && ["microphone", "useRecorder", "useLevelMeter", "usePitchDetector"].includes(binding.propertyName?.text ?? binding.name.text)) audioCapture = true;
          if (!binding.isTypeOnly && (binding.propertyName?.text ?? binding.name.text) === "usePlayer") {
            players.add(binding.name.text);
            audioPlayback = true;
          }
        }
      } else if (!statement.importClause?.isTypeOnly && bindings && ts.isNamespaceImport(bindings)) {
        audioCapture = true;
        audioPlayback = true;
        detachedAudio = true;
      }
    }
    if (ts.isImportDeclaration(statement) && statement.moduleSpecifier.text.startsWith("@ink/")) packages.add(statement.moduleSpecifier.text);
    if (ts.isImportDeclaration(statement) && /\.(?:png|jpe?g|webp)$/i.test(statement.moduleSpecifier.text)
      && statement.importClause?.name) imageImports.add(statement.importClause.name.text);
    if (ts.isImportDeclaration(statement) && statement.moduleSpecifier.text === "ink") {
      const bindings = statement.importClause?.namedBindings;
      if (bindings && ts.isNamedImports(bindings)) {
        for (const binding of bindings.elements) aliases.set(binding.name.text, binding.propertyName?.text ?? binding.name.text);
      }
    }
  }
  function visit(node) {
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && players.has(node.expression.text)) {
      const options = node.arguments[0];
      if (options && !ts.isObjectLiteralExpression(options)) detachedAudio = true;
      else if (options) {
        for (const property of options.properties) {
          if (ts.isSpreadAssignment(property)) detachedAudio = true;
          else if (property.name && ts.isComputedPropertyName(property.name)) detachedAudio = true;
          else if (property.name?.text === "mode") {
            if (!ts.isPropertyAssignment(property) || !ts.isStringLiteral(property.initializer)
              || property.initializer.text !== "attached") detachedAudio = true;
          }
        }
      }
    }
    if (ts.isStringLiteralLike(node)) strings.add(node.text);
    if (ts.isJsxOpeningElement(node) || ts.isJsxSelfClosingElement(node)) {
      const localTag = node.tagName.getText(source).split(".").at(-1);
      const tag = aliases.get(localTag) ?? localTag;
      components.add(tag);
      const attributes = node.attributes.properties;
      if (tag === "Image") {
        const src = attributes.find(attr => ts.isJsxAttribute(attr) && attr.name.getText(source) === "src");
        const value = src?.initializer;
        const expression = value && ts.isJsxExpression(value) ? value.expression : value;
        if (expression && ts.isStringLiteralLike(expression)) {
          if (expression.text.startsWith("https://")) remoteImages = true;
          else if (!expression.text.startsWith("asset://")) {
            replacements.push({ start: expression.getStart(source), end: expression.end, path: resolve(root, expression.text) });
          }
        } else if (!expression || !ts.isIdentifier(expression) || !imageImports.has(expression.text)) remoteImages = true;
      }
      const icon = attributes.find(attr => ts.isJsxAttribute(attr)
        && attr.name.getText(source) === (tag === "Icon" ? "name" : "icon"));
      if (icon || tag === "Icon" && attributes.some(ts.isJsxSpreadAttribute)) {
        const sizeAttribute = attributes.find(attr => ts.isJsxAttribute(attr) && attr.name.getText(source) === "size");
        const sizeExpression = sizeAttribute?.initializer;
        const numericSize = sizeExpression && ts.isJsxExpression(sizeExpression)
          && sizeExpression.expression && ts.isNumericLiteral(sizeExpression.expression)
          ? Number(sizeExpression.expression.text) : undefined;
        const size = tag === "Tab" ? 52 : tag === "Icon" ? numericSize ?? (sizeAttribute ? 52 : 28) : 30;
        const filled = tag === "Tab";
        const value = icon?.initializer;
        const expression = value && ts.isJsxExpression(value) ? value.expression : value;
        const names = iconNames(expression);
        if (names) {
          for (const name of names) addIcon(name, size, filled);
        } else {
          dynamicIcons.set(filled, Math.max(dynamicIcons.get(filled) ?? 0, size));
        }
      }
    }
    ts.forEachChild(node, visit);
  }
  visit(source);
  for (const replacement of replacements.sort((a, b) => b.start - a.start)) {
    contents = contents.slice(0, replacement.start) + JSON.stringify(await addAsset(replacement.path)) + contents.slice(replacement.end);
  }
  return contents;
}
const result = await Bun.build({
  entrypoints: [entry],
  target: "browser",
  format: "iife",
  minify: true,
  define: { "process.env.NODE_ENV": '"production"' },
  plugins: [{
    name: "ink-shared-dependencies",
    setup(build) {
      build.onLoad({ filter: /\.(?:png|jpe?g|webp|mp3)$/i }, async ({ path }) => ({
        contents: `export default ${JSON.stringify(await addAsset(path))}`,
        loader: "js",
      }));
      build.onResolve({ filter: /^(?:react|ink)(?:\/.*)?$/ }, ({ path }) => ({
        path: Bun.resolveSync(path, root),
      }));
      build.onLoad({ filter: /\.[cm]?[jt]sx?$/ }, async ({ path }) => {
        const contents = await inspectModule(path, await Bun.file(path).text());
        const extension = path.split(".").at(-1);
        const loader = extension === "tsx" || extension === "jsx" ? extension
          : extension === "ts" || extension === "mts" || extension === "cts" ? "ts" : "js";
        return { contents, loader };
      });
    },
  }],
});
if (!result.success) {
  for (const log of result.logs) console.error(log);
  process.exit(1);
}
await Bun.write(output, result.outputs[0]);
await Bun.write(`${output}.metadata.json`, JSON.stringify({
  components: [...components].sort(),
  packages: [...packages].sort(),
  assets: [...assets].map(([name, path]) => ({ name, path })),
  remoteImages,
  detachedAudio,
  audioPlayback,
  audioCapture,
  photoCapture,
  codeScanner,
  icons: [...icons.values()],
  dynamic: [...dynamicIcons].map(([filled, size]) => ({ filled, size })),
  strings: dynamicIcons.size ? [...strings].sort() : [],
}));
