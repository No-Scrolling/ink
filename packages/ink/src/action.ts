import { useCallback, useEffect, useRef, useState } from "react";

type ActionState<T> =
  | { status: "idle" }
  | { status: "pending" }
  | { status: "success"; data: T }
  | { status: "error"; error: Error };

export type Action<T, Args extends unknown[]> = ActionState<T> & {
  run: (...args: Args) => void;
};

export function useAction<T, Args extends unknown[]>(
  operation: (...args: Args) => T | PromiseLike<T>,
): Action<T, Args> {
  const [state, setState] = useState<ActionState<T>>({ status: "idle" });
  const mounted = useRef(false);
  const pending = useRef<object | null>(null);

  useEffect(() => {
    mounted.current = true;
    setState(current => current.status === "pending" ? { status: "idle" } : current);
    return () => {
      mounted.current = false;
      pending.current = null;
    };
  }, []);

  const run = useCallback((...args: Args) => {
    if (!mounted.current || pending.current) return;
    const request = {};
    pending.current = request;
    setState({ status: "pending" });
    const finish = (result: ActionState<T>) => {
      if (pending.current !== request) return;
      pending.current = null;
      setState(result);
    };
    void (async () => {
      try {
        finish({ status: "success", data: await operation(...args) });
      } catch (error) {
        finish({ status: "error", error: error instanceof Error ? error : new Error(String(error)) });
      }
    })();
  }, [operation]);

  return { ...state, run };
}
