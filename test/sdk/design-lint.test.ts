import { expect, test } from "bun:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "../..");

/** Lints pages with the SDK configuration that `ink check` uses and returns the ink-design diagnostics. */
async function lint(pages: Record<string, string>): Promise<string[]> {
  const directory = await mkdtemp(join(tmpdir(), "ink-design-lint-"));
  try {
    for (const [name, source] of Object.entries(pages)) await writeFile(join(directory, name), source);
    const result = Bun.spawnSync(
      ["bun", join(root, "node_modules/.bin/oxlint"), "--format=unix", "--config", join(root, "oxlint.config.mjs"), "."],
      { cwd: directory, stdout: "pipe", stderr: "pipe" },
    );
    const output = result.stdout.toString();
    if (result.exitCode !== 0 && !output.includes("[Error/")) throw new Error(result.stderr.toString() || output);
    return output
      .split("\n")
      .map(line => /^(?:\.\/)?([^:]+):(\d+):\d+: .*\[Error\/ink-design\(([^)]+)\)\]$/.exec(line))
      .filter(match => match !== null)
      .map(([, file, line, rule]) => `${file}:${line} ${rule}`)
      .sort();
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

test("ink check rejects the page structures DESIGN.md prohibits", async () => {
  expect(await lint({
    "setup.tsx": `import { Button, Screen, Stack } from "ink";
export function Setup({ ready, enable }: { ready: boolean; enable: () => void }) {
  return <Screen title="Set up">
    <Stack gap={24}>
      {ready ? <Button onPress={enable}>Enabled</Button> : <Button onPress={enable}>Enable</Button>}
    </Stack>
  </Screen>;
}
`,
    "settings.tsx": `import { Field, Screen } from "ink";
export function Settings() {
  return <Screen title="Settings">
    <Field label="Licences" href="/licences">{""}</Field>
    <Field label="About" href="/about" />
  </Screen>;
}
`,
    "backup.tsx": `import { Button, Screen, useAction } from "ink";
export function Backup({ save }: { save: () => Promise<void> }) {
  const transfer = useAction(save);
  const busy = transfer.status === "pending";
  return <Screen title="Backup">
    <Button disabled={busy} onPress={transfer.run}>Export</Button>
    <Button disabled={transfer.status === "pending" || false} onPress={transfer.run}>
      {transfer.status === "pending" ? "Exporting…" : "Export again"}
    </Button>
  </Screen>;
}
`,
  })).toEqual([
    "backup.tsx:6 no-pending-control-changes",
    "backup.tsx:7 no-pending-control-changes",
    "backup.tsx:8 no-pending-control-changes",
    "settings.tsx:4 no-empty-field",
    "settings.tsx:5 no-empty-field",
    "setup.tsx:4 no-page-wrapper-stack",
  ]);
});

test("ink check accepts the layouts DESIGN.md recommends", async () => {
  expect(await lint({
    "pages.tsx": `import { Button, Confirmation, Field, Screen, Stack, Text, useAction } from "ink";
export function Counter() {
  return <Screen title="Counter" centered>
    <Stack gap={16} align="center"><Text size={40}>3</Text><Button onPress={() => {}}>Increase</Button></Stack>
  </Screen>;
}
export function About() {
  return <Screen title="About">
    <Stack align="stretch" justify="space-between"><Button href="/licences">Licences</Button><Text>Ink</Text></Stack>
  </Screen>;
}
export function Forecast() {
  return <Screen title="London">
    <Stack gap={24}><Text size={88}>18°</Text><Text size={20}>Feels like 16°</Text></Stack>
  </Screen>;
}
export function Units({ unit, canChange }: { unit: string; canChange: boolean }) {
  return <Screen title="Units">
    <Field label="Temperature" href="/temperature">{unit}</Field>
    <Button disabled={!canChange} href="/advanced">Advanced</Button>
  </Screen>;
}
export function Watch({ start, stop }: { start: () => Promise<void>; stop: () => void }) {
  const watch = useAction(start);
  return <Screen title="Location">
    <Button onPress={() => (watch.status === "pending" ? stop() : watch.run())}>
      {watch.status === "pending" ? "Stop watch" : "Start watch"}
    </Button>
  </Screen>;
}
export function Delete({ remove }: { remove: () => Promise<void> }) {
  const action = useAction(remove);
  return <Confirmation title="Delete" confirmLabel="Delete" onConfirm={action.run}
    pending={action.status === "pending"} pendingLabel="Deleting…">Delete this list?</Confirmation>;
}
`,
    "local.tsx": `function Screen({ children }: { children: unknown }) { return children; }
function Stack({ children }: { children: unknown }) { return children; }
function Button({ children }: { children: unknown }) { return children; }
export const Page = () => <Screen><Stack><Button>Open</Button></Stack></Screen>;
`,
  })).toEqual([]);
});
