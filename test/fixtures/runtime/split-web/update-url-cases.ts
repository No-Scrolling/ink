// Freeze URL behaviour using the independent, published whatwg-url/tr46 parser.
// Run from the repository root with Bun. Native contracts consume the snapshots.
import { createRequire } from "node:module";
const require = createRequire(import.meta.resolve("../../../../packages/network/package.json"));
const { URL } = require("whatwg-url");
const revision = "8c17bfeb3e66a2986be16b0e004f85ace68db647";
const base = `https://raw.githubusercontent.com/web-platform-tests/wpt/${revision}/url/resources/`;
const read = async (name: string) => {
  const response = await fetch(base + name);
  if (!response.ok) throw new Error(`${name}: HTTP ${response.status}`);
  return response.json();
};
const urls = (await read("urltestdata.json")).filter((test: unknown) => typeof test === "object");
const setters = await read("setters_tests.json");
const cases = [
  ...urls.map((test: { input: string; base: string | null }) => ({ input: test.input, base: test.base })),
  ...["host", "hostname"].flatMap(property => setters[property].map((test: { href: string; new_value: string }) => ({ input: test.href, property, value: test.new_value }))),
];
const expected = cases.map(test => {
  try {
    const url = new URL(test.input, test.base ?? undefined);
    if (test.property) url[test.property] = test.value;
    return url.href;
  } catch { return null; }
});
await Bun.write(`${import.meta.dir}/url-cases.json`, JSON.stringify(cases) + "\n");
await Bun.write(`${import.meta.dir}/url-tr46-expected.json`, JSON.stringify(expected) + "\n");
console.log(`Froze ${cases.length} URL cases from WPT ${revision}`);
