import { expect, test } from "bun:test";
import ts from "typescript";
import { collectIconSizes } from "../../crates/ink-compiler/src/icon-usage.js";

const icons = new Map([["Add", "outlined:add"], ["Mail", "outlined:mail"], ["Check", "outlined:check"]]);
const svg = (specifier: string) => Promise.resolve(`svg:${specifier}`);
const sizes = (source: string) => collectIconSizes(ts, "icons.tsx", source, icons, svg);

test("known host usages select the largest required raster size across aliases", async () => {
  expect(await sizes(`
    import { Icon as Glyph, CanvasIcon, Button, Row, Message } from "ink";
    import { Add as Plus, Mail, Check } from "ink/icons";
    const page = <><Glyph name={Plus} size={18}/><CanvasIcon name={Plus} size={48}/>
      <Button icon={Mail}/><Row subtitleIcon={Check}/><Message statusIcon={Check}/></>;
  `)).toEqual(new Map([["outlined:add", 48], ["outlined:mail", 40], ["outlined:check", 16]]));
});

test("unknown uses, dynamic sizes and spreads retain default resolution", async () => {
  expect(await sizes(`
    import { Icon } from "ink";
    import { Add, Mail, Check } from "ink/icons";
    const use = <><Icon name={Add} size={12}/><Custom icon={Add}/>
      <Icon name={Mail} size={setting}/><Icon name={Check} size={16} {...props}/></>;
  `)).toEqual(new Map([["outlined:add", 56], ["outlined:mail", 56], ["outlined:check", 56]]));
});

test("shadowed import names and unused or type-only imports do not inflate assets", async () => {
  expect(await sizes(`
    import { Icon } from "ink";
    import { Add, Mail, type Check } from "ink/icons";
    function local(Add) { return <Icon name={Add} size={100}/>; }
    const page = <Icon name={Add}/>;
  `)).toEqual(new Map([["outlined:add", 28]]));
});

test("SVG references share the same size policy and exports preserve reusable asset resolution", async () => {
  expect(await sizes(`import { Icon } from "ink"; import Mark from "./mark.svg";
    const page = <Icon name={Mark} size={22}/>;`)).toEqual(new Map([["svg:./mark.svg", 22]]));
  expect(await sizes('export { default as Mark } from "./mark.svg";')).toEqual(new Map([["svg:./mark.svg", 56]]));
  expect(await sizes('export { Add } from "ink/icons";')).toEqual(new Map([
    ["outlined:add", 56], ["outlined:mail", 56], ["outlined:check", 56],
  ]));
});
