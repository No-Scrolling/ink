import { readdir } from "node:fs/promises";
import { resolve } from "node:path";

export async function generateFileRoutes(root) {
  const imports = ['import { FileNavigator } from "ink/file-router";'];
  const routes = new Set();
  const shapes = new Set();
  let nextId = 0;
  function component(path) {
    const name = `Page${nextId++}`;
    imports.push(`import ${name} from ${JSON.stringify(path)};`);
    return name;
  }
  async function directory(path, segments, name) {
    const entries = (await readdir(path, { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name));
    const pages = [];
    const children = [];
    let layout;
    for (const entry of entries) {
      if (entry.name.startsWith(".") || entry.name.startsWith("_") && entry.name !== "_layout.tsx") continue;
      const file = resolve(path, entry.name);
      const segment = entry.isDirectory() ? entry.name : entry.name.replace(/\.tsx$/, "");
      if (/[\[\]$]/.test(segment) && !/^\[[A-Za-z_][A-Za-z0-9_]*\]$/.test(segment)) {
        throw new Error(`Use a single named parameter, such as [id], in ${file}`);
      }
      if (entry.isDirectory()) {
        const group = /^\(.+\)$/.test(entry.name);
        children.push(await directory(file, group ? segments : [...segments, entry.name], entry.name));
      } else if (entry.name === "_layout.tsx") {
        layout = component(file);
      } else if (entry.name.endsWith(".tsx")) {
        const name = entry.name.slice(0, -4);
        const route = "/" + [...segments, ...(name === "index" ? [] : [name])].join("/");
        if (routes.has(route)) throw new Error(`Duplicate file route: ${route}`);
        const parameters = [...route.matchAll(/\[([^\]]+)\]/g)].map(match => match[1]);
        if (new Set(parameters).size !== parameters.length) throw new Error(`Repeated route parameter: ${route}`);
        const shape = route.replace(/\[[^\]]+\]/g, "[]");
        if (shapes.has(shape)) throw new Error(`Ambiguous file route: ${route}`);
        shapes.add(shape);
        routes.add(route);
        pages.push(`{ name: ${JSON.stringify(name)}, path: ${JSON.stringify(route)}, component: ${component(file)} }`);
      }
    }
    return `{ name: ${JSON.stringify(name)}, ${layout ? `layout: ${layout}, ` : ""}pages: [${pages.join(",")}], children: [${children.join(",")}] }`;
  }
  const tree = await directory(resolve(root, "app"), [], "app");
  if (!routes.has("/")) throw new Error("File routing requires app/index.tsx or an index.tsx inside a route group");
  const source = imports.join("\n") + `\nconst tree = ${tree};\nexport default function App() { return <FileNavigator tree={tree} />; }\n`;
  const output = resolve(root, ".ink/routes.tsx");
  if (!await Bun.file(output).exists() || await Bun.file(output).text() !== source) await Bun.write(output, source);
}
