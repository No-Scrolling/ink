import { Binding } from "./view";
import type { ViewData } from "./host";

export class Expression<T extends ViewData> {
  declare readonly result: T;
  constructor(readonly definition: ViewData) {}
}
type Operand<T extends ViewData> = T | Binding<T> | Expression<T>;
function node<T extends ViewData>(value: Operand<T>): ViewData {
  if (value instanceof Expression) return value.definition;
  if (value instanceof Binding) return { op: "value", source: value.source, path: value.path };
  return { op: "literal", value };
}
function binary<T extends ViewData>(op: string, left: Operand<number>, right: Operand<number>) {
  return new Expression<T>({ op, args: [node(left), node(right)] });
}

/** Native expressions accept finite numbers and JSON values, without JS coercion. */
export const expression = {
  add: (left: Operand<number>, right: Operand<number>) => binary<number>("add", left, right),
  subtract: (left: Operand<number>, right: Operand<number>) => binary<number>("subtract", left, right),
  multiply: (left: Operand<number>, right: Operand<number>) => binary<number>("multiply", left, right),
  divide: (left: Operand<number>, right: Operand<number>) => binary<number>("divide", left, right),
  remainder: (left: Operand<number>, right: Operand<number>) => binary<number>("remainder", left, right),
  less: (left: Operand<number>, right: Operand<number>) => binary<boolean>("less", left, right),
  equal: <T extends ViewData>(left: Operand<T>, right: Operand<T>) => new Expression<boolean>({ op: "equal", args: [node(left), node(right)] }),
  not: (value: Operand<boolean>) => new Expression<boolean>({ op: "not", value: node(value) }),
  choose: <T extends ViewData>(condition: Operand<boolean>, yes: Operand<T>, no: Operand<T>) =>
    new Expression<T>({ op: "choose", condition: node(condition), yes: node(yes), no: node(no) }),
  concat: (...parts: Operand<string | number | boolean | null>[]) => new Expression<string>({ op: "concat", parts: parts.map(node) }),
  length: (value: Operand<string | readonly ViewData[]>) => new Expression<number>({ op: "length", value: node(value) }),
  pad: (value: Operand<number | string>, width: number) => new Expression<string>({ op: "pad", value: node(value), width }),
};
