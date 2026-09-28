// Diagnostic only: fixed native controls, JavaScript state and calculations.
// This is not a general React-compatible renderer.
let nextId = 1, count = 100, tick = 0, reverse = false, hidden = false, resize = false;
let operations = [], body, summary, hiddenLabel, resizeLabel, rows = [], labels = [];
const handlers = new Map();
function node(type, props, parent) {
  const id = nextId++;
  operations.push({op: 'create', id, type, props});
  if (parent !== undefined) operations.push({op: 'insert', id, parent, before: null});
  return id;
}
function button(value, props, parent, action) {
  const id = node('Text', {...props, text: value, onPress: true}, parent);
  handlers.set(id, action); return id;
}
function values() {
  const ids = Array.from({length: count}, (_, id) => id);
  if (reverse) ids.reverse();
  return Array.from({length: Math.ceil(count / 10)}, (_, row) => ids.slice(row * 10, row * 10 + 10));
}
function flush() { if (operations.length) __inkCommit(operations); operations = []; }
const screen = node('Screen', {title: 'Update benchmark', pinnedHeader: true, rightIcon: 'outlined:refresh', onRightPress: true}, 0);
const headerWrapper = node('Stack', {gap: 47}, screen);
const header = node('Stack', {gap: 8}, headerWrapper);
const sizes = node('Stack', {axis: 'horizontal', gap: 16}, header);
for (const value of [1, 10, 100, 500]) button(String(value), {size: 18, width: 50}, sizes, () => {
  count = value; tick = 0; rebuild(); update();
});
const controls = node('Stack', {axis: 'horizontal', gap: 16}, header);
button('Reverse', {size: 14, width: 85}, controls, () => { reverse = !reverse; update(); });
hiddenLabel = button('Hide', {size: 14, width: 85}, controls, () => {
  hidden = !hidden;
  operations.push({op: 'hidden', id: body, value: hidden}, {op: 'text', changes: [hiddenLabel, hidden ? 'Show' : 'Hide']});
});
resizeLabel = button('Resize off', {size: 14, width: 85}, controls, () => {
  resize = !resize;
  operations.push({op: 'text', changes: [resizeLabel, resize ? 'Resize on' : 'Resize off']});
  update();
});
summary = node('Text', {size: 14, text: `${count} cells · ${tick} updates`}, header);
function rebuild() {
  if (body !== undefined) operations.push({op: 'remove', id: body, parent: screen});
  body = node('Stack', {gap: 4}, screen); rows = []; labels = [];
  for (const row of values()) {
    const gap = resize ? tick % 2 : 0;
    const id = node('Stack', {axis: 'horizontal', gap}, body); rows.push({id, gap});
    for (const value of row) {
      const text = `${String(value).padStart(3, '0')}:${(value + tick) % 100}`;
      labels.push({id: node('Text', {size: 8, width: 30, maxLines: 1, tabularNumbers: true, text}, id), text});
    }
  }
  if (hidden) operations.push({op: 'hidden', id: body, value: true});
}
function update() {
  const changes = [summary, `${count} cells · ${tick} updates`];
  let index = 0;
  for (const [rowIndex, row] of values().entries()) {
    const gap = resize ? tick % 2 : 0, nativeRow = rows[rowIndex];
    if (nativeRow.gap !== gap) {
      operations.push({op: 'update', id: nativeRow.id, props: {axis: 'horizontal', gap}});
      nativeRow.gap = gap;
    }
    for (const value of row) {
      const label = labels[index++], text = `${String(value).padStart(3, '0')}:${(value + tick) % 100}`;
      if (text !== label.text) { changes.push(label.id, text); label.text = text; }
    }
  }
  operations.push({op: 'text', changes});
}
globalThis.__inkReceive = raw => {
  const message = JSON.parse(raw);
  if (message.type !== 'event') return;
  if (message.id === screen && message.name === 'onRightPress') { tick++; update(); }
  else if (message.name === 'onPress') handlers.get(message.id)?.();
  flush();
};
rebuild(); flush();
