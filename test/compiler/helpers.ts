import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import ts from "typescript";
import { compileNativeLists } from "../../crates/ink-compiler/src/native-lists.js";

export async function loadCompiled(source: string) {
  const directory = await mkdtemp(resolve(tmpdir(), "ink-compiled-row-"));
  try {
    const path = resolve(directory, "row.tsx");
    const compiled = compileNativeLists(ts, path, source);
    let javascript = ts.transpileModule(compiled, {
      compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ESNext, jsx: ts.JsxEmit.ReactJSX },
    }).outputText;
    // Bun's isolated workspace install does not hoist these dependencies to the
    // repository root. Resolve the genuine modules from their owning package.
    for (const [specifier, target] of Object.entries({
      "ink": resolve(import.meta.dir, "../../packages/ink/src/index.ts"),
      "ink/internal/list": resolve(import.meta.dir, "../../packages/ink/src/native-list.ts"),
      "react/jsx-runtime": Bun.resolveSync("react/jsx-runtime", resolve(import.meta.dir, "../../packages/ink")),
    })) javascript = javascript.replaceAll(JSON.stringify(specifier), JSON.stringify(target));
    const output = resolve(directory, "row.js");
    await writeFile(output, javascript);
    return { module: await import(output), compiled };
  } finally { await rm(directory, { recursive: true, force: true }); }
}
