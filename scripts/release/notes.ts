import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";

const sdk = JSON.parse(await readFile(new URL("../../sdk.json", import.meta.url), "utf8"));
const tag = `v${process.argv[2] ?? sdk.version}`;
const repository = `https://github.com/${sdk.repository}`;
const git = (...args) => execFileSync("git", args, { encoding: "utf8" }).trim();
const previous = git("tag", "--merged", tag, "--sort=-version:refname")
  .split("\n")
  .find((name) => /^v\d+\.\d+\.\d+/.test(name) && name !== tag);
const commits = git("log", "--no-merges", "--reverse", "--format=%H %s", previous ? `${previous}..${tag}` : tag)
  .split("\n")
  .filter(Boolean)
  .map((line) => {
    const separator = line.indexOf(" ");
    return { hash: line.slice(0, separator), subject: line.slice(separator + 1) };
  })
  .filter(({ subject }) => !/^release: v\d+\.\d+\.\d+(?:-[\w.-]+)?$/.test(subject));

console.log("## What's changed\n");
for (const { hash, subject } of commits) {
  const title = subject.replace(/[\\`*_{}\[\]<>]/g, "\\$&");
  console.log(`- ${title} ([${hash.slice(0, 7)}](${repository}/commit/${hash}))`);
}
if (!commits.length) console.log("No changes beyond release metadata.");
const changelog = previous ? `${repository}/compare/${previous}...${tag}` : `${repository}/commits/${tag}`;
console.log(`\n[Full changelog](${changelog})`);
