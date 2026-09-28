import { Activity, createElement, useState } from 'react';
import { render } from '../../packages/ink/src/renderer';
import { nativeView, type ViewValue } from '../../packages/ink/src/view';
declare const __inkReceive: (message: string) => void;
declare let __inkCommit: (operations: Record<string, unknown>[]) => void;
declare const __inkCompatResult: (message: string) => void;
const actualCommit = __inkCommit;
const commits: unknown[][] = [];
__inkCommit = operations => {
  actualCommit(operations);
  commits.push(operations.map(op => op.op === 'text' ? {
    op: 'text', ids: (op.changes as unknown[]).filter((_, i) => i % 2 === 0),
    values: (op.changes as unknown[]).filter((_, i) => i % 2 === 1),
  } : op));
};
let source: ViewValue<number>;
let hidden!: (value: boolean) => void;
let mounted!: (value: boolean) => void;
let starts = 0, stops = 0;
const Page = nativeView(view => {
  source = view.value(0);
  const label = view.derive([source], () => `Value ${source.get()}`);
  view.onMount(() => { starts++; if (starts === 3) source.set(20); return () => { stops++; }; });
  return view.node('Screen', {title:'Bindings'}, view.node('Text', {text:label, onPress:() => source.set(source.get()+1)}));
});
function App() {
  const [hide, setHide] = useState(false), [show, setShow] = useState(true);
  hidden = setHide; mounted = setShow;
  return show ? createElement(Activity, {mode:hide?'hidden':'visible'}, createElement(Page)) : null;
}
const settle = () => new Promise(resolve => setTimeout(resolve, 15));
function assert(value: boolean, message: string) { if (!value) throw Error(message); }
function actionId() {
  const create = commits.flat().find((op: any) => op.op === 'create' && op.type === 'NativeView') as any;
  return JSON.parse(create.props.definition).root.children[0].id;
}
function hasLabel(label: string) { return JSON.stringify(commits.at(-1)).includes(label); }
render(createElement(App)); await settle();
assert(starts===1 && stops===0, 'initial lifetime');
const id = actionId();
__inkReceive(JSON.stringify({type:'event',id,name:'onPress',args:[]})); await settle();
assert(hasLabel('Value 1'), 'native action updates binding');
source!.set(2); source!.set(3); await settle(); assert(hasLabel('Value 3'), 'batch takes latest value');
hidden(true); await settle(); const before = commits.length;
source!.set(9); await settle(); assert(commits.length===before, 'hidden view does not publish values');
hidden(false); await settle(); assert(hasLabel('Value 9'), 'reveal catches up');
assert(starts===2 && stops===1, 'Activity reconnects lifetime');
mounted(false); await settle(); const removed = commits.length;
source!.set(11); __inkReceive(JSON.stringify({type:'event',id,name:'onPress',args:[]})); await settle();
assert(commits.length===removed, 'unmounted values and callbacks cannot publish');
mounted(true); await settle(); assert(starts===3 && stops===2, 'fresh mount');
__inkCompatResult(JSON.stringify({commits,checks:['action','batch','Activity hide/reveal','unmount','stale callbacks','remount']}));
