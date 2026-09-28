function equal(a, b) { if (!Object.is(a, b)) throw Error(`${a} != ${b}`); }
for (let i = 0; i < 2000; i++) {
  const value = {a: i, b: i + 1, c: {x: i, y: i + 2}};
  equal(value.a, i); equal(value.b, i + 1); equal(value.c.y, i + 2);
  delete value.a; value.a = 17; equal(Object.keys(value).join(','), 'b,c,a');
  const log = [];
  const duplicate = {a: (log.push(1), i), b: (log.push(2), i + 1), a: (log.push(3), i + 2)};
  equal(duplicate.a, i + 2); equal(log.join(','), '1,2,3'); equal(Object.keys(duplicate).join(','), 'a,b');
  const callback = {a: () => i, b: function () { return i + 1; }};
  equal(callback.a.name, 'a'); equal(callback.b.name, 'b'); equal(callback.a(), i);
  const getters = {a: i, get b() { return this.a + 1; }, set b(x) { this.a = x - 1; }};
  getters.b = 5; equal(getters.a, 4);
  const method = {a: i, b() { return this.a; }}; equal(method.b(), i);
  const computed = {a: i, ['x' + i]: i + 1}; equal(computed['x' + i], i + 1);
  const spread = {a: i, ...{b: i + 1}, c: i + 2}; equal(spread.c, i + 2);
  const proto = {__proto__: null, a: i, b: i + 1}; equal(Object.getPrototypeOf(proto), null);
  const symbol = Symbol(); const keyed = {a: i, [symbol]: i + 1}; equal(keyed[symbol], i + 1);
  const numeric = {a: i, 1: i, 0: i + 1}; equal(Object.keys(numeric).join(','), '0,1,a');
  let order = 0; try { const interrupted = {a: ++order, b: (() => { throw 42; })(), c: ++order}; } catch(e) { equal(e, 42); } equal(order, 1);
  const collected = {a: {v: i, w: i}, b: (gc(), i)}; equal(collected.a.v, i);
}
function* make() { return {a: yield 1, b: yield 2}; }
const generator = make(); equal(generator.next().value, 1); equal(generator.next(11).value, 2);
const result = generator.next(22).value; equal(result.a, 11); equal(result.b, 22);
async function deferred() { return {a: await 4, b: await 7}; }
deferred().then(value => { equal(value.a, 4); equal(value.b, 7); });
const many = Function('return {' + Array.from({length: 40}, (_, i) => `field${i}:${i}`).join(',') + '}')(); equal(many.field39, 39);
let reads = 0; const trap = {get a() { reads++; return 3; }}; equal(({a: trap.a, b: 4}).a, 3); equal(reads, 1);
