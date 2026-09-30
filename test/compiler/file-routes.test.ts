import { afterAll, expect, test } from "bun:test";
import { mkdtemp, mkdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import ts from "typescript";
import { generateFileRoutes } from "../../crates/ink-compiler/src/file-routes.js";

const directories: string[] = [];
afterAll(() => Promise.all(directories.map(path => rm(path, { recursive: true, force: true }))));
async function app(files: string[]) {
  const root = await mkdtemp(resolve(tmpdir(), "ink-routes-"));
  directories.push(root);
  await mkdir(resolve(root, ".ink"));
  for (const file of files) {
    const path = resolve(root, "app", file);
    await mkdir(dirname(path), { recursive: true });
    await writeFile(path, "export default function Page() { return null; }\n");
  }
  return root;
}

// Read the generated data structurally rather than depending on formatting or
// synthetic Page numbers. Import paths remain the actual generated values.
function routeTree(contents: string) {
  const source = ts.createSourceFile("routes.tsx", contents, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  const imports = new Map<string, string>();
  for (const statement of source.statements) {
    if (ts.isImportDeclaration(statement) && statement.importClause?.name && ts.isStringLiteral(statement.moduleSpecifier)) {
      imports.set(statement.importClause.name.text, statement.moduleSpecifier.text);
    }
  }
  function literal(node: ts.Node): unknown {
    if (ts.isStringLiteral(node)) return node.text;
    if (ts.isIdentifier(node)) return imports.get(node.text);
    if (ts.isArrayLiteralExpression(node)) return node.elements.map(literal);
    if (ts.isObjectLiteralExpression(node)) return Object.fromEntries(node.properties.map(property => {
      if (!ts.isPropertyAssignment(property)) throw Error("Expected route data property");
      return [property.name.getText(source), literal(property.initializer)];
    }));
    throw Error(`Unexpected route data: ${node.getText(source)}`);
  }
  for (const statement of source.statements) {
    if (!ts.isVariableStatement(statement)) continue;
    const tree = statement.declarationList.declarations.find(declaration => declaration.name.getText(source) === "tree");
    if (tree?.initializer) return literal(tree.initializer);
  }
  throw Error("Generated routes have no tree");
}

test("route groups preserve layout ancestry without adding URL segments", async () => {
  const root = await app(["_layout.tsx", "(main)/_layout.tsx", "(main)/index.tsx", "(main)/notes/[id].tsx", "settings/index.tsx", "_private.tsx", ".hidden.tsx", "notes.txt"]);
  await generateFileRoutes(root);
  const generated = await readFile(resolve(root, ".ink/routes.tsx"), "utf8");
  expect(routeTree(generated)).toEqual({
    name: "app", layout: resolve(root, "app/_layout.tsx"), pages: [], children: [
      { name: "(main)", layout: resolve(root, "app/(main)/_layout.tsx"), pages: [
        { name: "index", path: "/", component: resolve(root, "app/(main)/index.tsx") },
      ], children: [
        { name: "notes", pages: [{ name: "[id]", path: "/notes/[id]", component: resolve(root, "app/(main)/notes/[id].tsx") }], children: [] },
      ] },
      { name: "settings", pages: [{ name: "index", path: "/settings", component: resolve(root, "app/settings/index.tsx") }], children: [] },
    ],
  });
  expect(generated).toContain('<FileNavigator tree={tree} />');
  const before = await stat(resolve(root, ".ink/routes.tsx"));
  await generateFileRoutes(root);
  const after = await stat(resolve(root, ".ink/routes.tsx"));
  expect(after.mtimeMs).toBe(before.mtimeMs);
  expect(await readFile(resolve(root, ".ink/routes.tsx"), "utf8")).toBe(generated);
});

test.each([
  { files: ["index.tsx", "(group)/index.tsx"], message: "Duplicate file route: /" },
  { files: ["index.tsx", "notes/[id].tsx", "notes/[name].tsx"], message: "Ambiguous file route:" },
  { files: ["index.tsx", "[id]/[id].tsx"], message: "Repeated route parameter: /[id]/[id]" },
  { files: ["index.tsx", "[...all].tsx"], message: "Use a single named parameter" },
  { files: ["index.tsx", "[[id]].tsx"], message: "Use a single named parameter" },
  { files: ["index.tsx", "[9id].tsx"], message: "Use a single named parameter" },
  { files: ["about.tsx"], message: "File routing requires app/index.tsx" },
])("rejects an invalid route graph: $message ($files)", async ({ files, message }) => {
  const root = await app(files);
  await expect(generateFileRoutes(root)).rejects.toThrow(message);
  expect(await Bun.file(resolve(root, ".ink/routes.tsx")).exists()).toBe(false);
});
