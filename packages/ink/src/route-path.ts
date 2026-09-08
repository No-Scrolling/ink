export type PathParams<Path extends string> =
  Path extends `${string}[${infer Name}]${infer Rest}`
    ? { [Key in Name | keyof PathParams<Rest>]: string }
    : Record<never, never>;

export function parameterNames(path: string): string[] {
  return [...path.matchAll(/\[([A-Za-z_][A-Za-z0-9_]*)\]/g)].map(match => match[1]);
}

export function matchPath(pattern: string, path: string): Record<string, string> | undefined {
  const expected = pattern.split("/");
  const actual = path.split("/");
  if (expected.length !== actual.length) return;
  const params: Record<string, string> = {};
  for (let index = 0; index < expected.length; index++) {
    const name = /^\[([A-Za-z_][A-Za-z0-9_]*)\]$/.exec(expected[index])?.[1];
    if (name) {
      if (!actual[index]) return;
      try { params[name] = decodeURIComponent(actual[index]); } catch { return; }
    } else if (expected[index] !== actual[index]) return;
  }
  return params;
}

// Compare each segment so a static name wins over a parameter at the same depth.
export function comparePaths(left: string, right: string): number {
  const a = left.split("/");
  const b = right.split("/");
  for (let index = 0; index < Math.min(a.length, b.length); index++) {
    const difference = Number(a[index].startsWith("[")) - Number(b[index].startsWith("["));
    if (difference) return difference;
  }
  return left.localeCompare(right);
}
