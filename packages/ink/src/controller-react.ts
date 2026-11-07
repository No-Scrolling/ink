import { useMemo } from "./react";
import { attachNativeController } from "./controller";

type Attachment = ReturnType<typeof attachNativeController>;
type ControllerAttachment = {
  current: Attachment | null;
  attach(recipe: unknown, observe: (value: unknown) => void, fail: (error: Error) => void,
    ready?: (id: number) => void): () => void;
};

/** @internal Attach from an effect and return its cleanup, including for hidden Activities. */
export function useNativeController(module: string, description = module): ControllerAttachment {
  return useMemo(() => {
    const controller: ControllerAttachment = {
      current: null,
      attach(recipe, observe, fail, ready) {
        const attachment = attachNativeController(module, recipe, observe);
        controller.current = attachment;
        // Disposal removes event observers; promise continuations need their own lifetime check.
        void attachment.ready.then(() => {
          if (controller.current === attachment) ready?.(attachment.id);
        }, error => {
          if (controller.current === attachment) fail(error);
        });
        return () => {
          controller.current = null;
          void attachment.dispose().catch(error => console.error(`Could not release ${description}`, error));
        };
      },
    };
    return controller;
  }, [module, description]);
}
