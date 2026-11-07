import { useMemo } from "preact/hooks";

/** React Compiler's per-component memo slots, retained across Preact renders. */
export function c(size: number): unknown[] {
  return useMemo(() => Array(size).fill(Symbol.for("react.memo_cache_sentinel")), []);
}
