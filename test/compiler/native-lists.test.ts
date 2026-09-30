import { expect, test } from "bun:test";
import { nativeListPlan } from "../../packages/ink/src/native-list";
import { loadCompiled as load } from "./helpers";

test("emitted template binds nested content and properties to the correct projected values", async () => {
  const { module } = await load(`
    import { List, Stack, Text, Toggle, Button } from "ink";
    export function make(suffix, events) {
      return <List items={[]} keyExtractor={row => row.id}
        renderItem={(row, index) => <Stack gap={row.gap}>
          <Text size={20}>{index + 1}: {row.name}{suffix}{false}{null}{row.parts}</Text>
          <Toggle checked={row.enabled} onChange={value => events.push([row.id, value])} />
          <Button onPress={() => events.push(row.id)}>Open {row.name}</Button>
        </Stack>} />;
    }
  `);
  const events: unknown[] = [];
  const element = module.make("!", events);
  const render = element.props.renderItem;
  const plan = nativeListPlan(render)!;
  expect(plan).toBeDefined();
  const row = { id: "r", name: "Ada", gap: 8, enabled: true, parts: ["/", 3, false, ["x"]] };
  const values = plan.project(row, 4);
  const template = JSON.parse(plan.template);
  expect(template.children[0].type).toBe("Stack");
  const [text, toggle, button] = template.children[0].children;
  const at = (binding: { $value: number; path: (string | number)[] }) => {
    expect(binding.$value).toBe(0);
    return binding.path.reduce<unknown>((value, key) => {
      if (typeof value !== "object" || value === null) throw Error("Binding must traverse projected data");
      return Reflect.get(value, key);
    }, { values });
  };
  const invoke = (binding: { $value: number; path: (string | number)[] }, ...args: unknown[]) => {
    const callback = at(binding);
    expect(typeof callback).toBe("function");
    if (typeof callback !== "function") throw Error("Expected a projected action");
    callback(...args);
  };
  expect(at(template.children[0].props.gap)).toBe(8);
  expect(at(text.props.size)).toBe(20);
  expect(at(text.props.text)).toBe("5: Ada!/3x");
  expect(at(toggle.props.checked)).toBe(true);
  expect(button.children[0].type).toBe("RawText");
  expect(at(button.children[0].props.text)).toBe("Open Ada");
  expect(events).toEqual([]);
  invoke(toggle.props.onChange, false);
  invoke(button.props.onPress);
  expect(events).toEqual([["r", false], "r"]);
  expect(element.props.keyExtractor(row)).toBe("r");
  // Root identifiers from property accesses are captured, not field names or
  // callback parameters. Reading a capture later must observe current state.
  expect(plan.scope!.values()).toEqual(["!"]);
});

test("Row lowering preserves presentation and action semantics", async () => {
  const { module } = await load(`
    import { List, Row } from "ink";
    export const make = action => <List renderItem={row => <Row title={row.name}
      subtitle="Saved" titleMaxLines={2} image={row.image} onLongPress={action} />} />;
  `);
  let pressed = 0;
  const action = () => { pressed++; };
  const plan = nativeListPlan(module.make(action).props.renderItem)!;
  const fields = plan.project({ name: "Note", image: "asset://image" }, 0)[0] as {
    title: string; subtitle: string; titleMaxLines: number; image: string;
    onPress: () => void; onLongPress: () => void;
  };
  expect(fields.title).toBe("Note");
  expect(fields.subtitle).toBe("Saved");
  expect(fields.titleMaxLines).toBe(2);
  expect(fields.image).toBe("asset://image");
  expect(pressed).toBe(0);
  fields.onPress();
  expect(pressed).toBe(0);
  fields.onLongPress();
  expect(pressed).toBe(1);
  const root = JSON.parse(plan.template).children[0];
  expect(root.type).toBe("RowContent");
  expect(root.props.onLongPress).toEqual({ $value: 0, path: ["values", 0, "onLongPress"] });
});

test("lowering several lists avoids source identifier collisions and binds each closure independently", async () => {
  const { module, compiled } = await load(`
    import { List, Text } from "ink";
    const __inkList = "occupied", __inkList_ = "also occupied";
    export function make(label) {
      return [<List renderItem={row => <Text>{label}{row.name}</Text>} />,
        <List renderItem={row => <Text>{__inkList}{__inkList_}{row.name}</Text>} />];
    }
  `);
  const [first, second] = module.make("A:");
  expect(compiled).toContain("nativeListRow as __inkList__Row");
  expect(nativeListPlan(first.props.renderItem)!.project({ name: "One" }, 0)).toEqual(["A:One"]);
  expect(nativeListPlan(second.props.renderItem)!.project({ name: "Two" }, 0)).toEqual(["occupiedalso occupiedTwo"]);
  expect(nativeListPlan(module.make("B:")[0].props.renderItem)!.project({ name: "One" }, 0)).toEqual(["B:One"]);
});

test("one ineligible row does not prevent another list from being lowered", async () => {
  const { module } = await load(`
    import { List, Text } from "ink";
    export const make = format => [<List renderItem={row => <Text>{format(row)}</Text>} />,
      <List renderItem={row => <Text>{row.name}</Text>} />];
  `);
  let calls = 0;
  const [fallback, lowered] = module.make(() => { calls++; return "Formatted"; });
  expect(nativeListPlan(fallback.props.renderItem)).toBeUndefined();
  expect(nativeListPlan(lowered.props.renderItem)!.project({ name: "Plain" }, 0)).toEqual(["Plain"]);
  expect(calls).toBe(0);
  expect(fallback.props.renderItem({}).props.children).toBe("Formatted");
  expect(calls).toBe(1);
});

test("inline text lowering preserves the ordinary JSX callback text", async () => {
  const { module } = await load('import { List, Text } from "ink"; export const row = <List renderItem={item => <Text>A\tB{item.name}</Text>} />;');
  const render = module.row.props.renderItem;
  const plan = nativeListPlan(render)!;
  const regularChildren = render({ name: "C" }, 0).props.children;
  expect(regularChildren).toEqual(["A\tB", "C"]);
  expect(plan.project({ name: "C" }, 0)).toEqual(["A\tBC"]);
});
