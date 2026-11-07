import "@prefresh/core";
import { flush } from "@prefresh/utils";

declare const self: { __PREFRESH__: {
  register(type: unknown, id: string): void;
  sign(type: unknown, key: string, forceReset: boolean, getCustomHooks: (() => unknown[]) | undefined, status: string): string;
} };

export const register = (type: unknown, id: string) => self.__PREFRESH__.register(type, id);
export const performReactRefresh = flush;

// Oxc emits React Refresh's signature protocol; translate it to Prefresh.
export function createSignatureFunctionForTransform() {
  let savedType: unknown;
  let savedKey = "";
  let savedReset = false;
  let savedHooks: (() => unknown[]) | undefined;
  let status = "begin";
  return (type?: unknown, key?: string, forceReset = false, getCustomHooks?: () => unknown[]) => {
    if (type) {
      savedType = type;
      savedKey = key ?? "";
      savedReset = forceReset;
      savedHooks = getCustomHooks;
    }
    status = self.__PREFRESH__.sign(savedType, savedKey, savedReset, savedHooks, status);
    return type;
  };
}
