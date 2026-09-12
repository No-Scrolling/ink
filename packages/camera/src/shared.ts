import { createElement, useEffect, useMemo, useRef, useState } from "react";
import { Button, Field, Image, Stack, useAction } from "ink";
import { callNative, NativeError } from "ink/native";
import { attachNativeController } from "ink/native/controller";
import type { FileRef } from "@ink/files";

export const camera = {
  removePhoto: async (file: CapturedFile): Promise<void> => {
    await callNative("camera", "remove-photo", { source: file.source });
  },
};
export const codeFormats = ["qr", "aztec", "data-matrix", "pdf417", "codabar", "code-39", "code-93", "code-128", "ean-8", "ean-13", "itf", "upc-a", "upc-e"] as const;
export type CodeFormat = typeof codeFormats[number];
export interface CodeScan { text: string; format: CodeFormat; rawBytes: Uint8Array | null }
export interface CapturedFile extends FileRef { uri: string; source: string; mimeType: "image/jpeg" }
export interface CapturedPhoto { file: CapturedFile; source: string; width: number; height: number; mimeType: "image/jpeg"; capturedAt: number }
interface CameraState<T> {
  status: "idle" | "opening" | "active" | "review" | "ready" | "error";
  ready: boolean;
  value: T | null;
  reviewSource: string | null;
  saving: boolean;
  error: Error | null;
}
function initial<T>(): CameraState<T> { return { status: "idle", ready: false, value: null, reviewSource: null, saving: false, error: null }; }
export function useSession<T>(kind: "photo" | "scanner", decode: (value: unknown) => T, config = "{}") {
  const [state, setState] = useState<CameraState<T>>(initial);
  const [previewId, setPreviewId] = useState<number | null>(null);
  const controller = useRef<ReturnType<typeof attachNativeController> | null>(null);
  useEffect(() => {
    setState(initial());
    setPreviewId(null);
    const fail = (error: unknown) => setState({ ...initial<T>(), status: "error", error: error instanceof Error ? error : new Error(String(error)) });
    const attachment = attachNativeController("camera", { kind, config: JSON.parse(config) }, value => {
      try {
        if (typeof value !== "object" || value === null || !("status" in value)) throw new NativeError("protocol", "Invalid camera state");
        if ("reviewSource" in value) {
          if (typeof value.reviewSource !== "string") throw new NativeError("protocol", "Invalid photo review source");
          const source = value.reviewSource;
          setState(previous => source ? { ...previous, status: "review", ready: false, reviewSource: source, saving: "saving" in value && value.saving === true }
            : previous.status === "review" ? { ...initial<T>(), status: "active", ready: true } : previous);
          return;
        }
        const status = value.status;
        if (status === "error") {
          if (!("error" in value) || typeof value.error !== "object" || value.error === null
            || !("kind" in value.error) || typeof value.error.kind !== "string"
            || !("message" in value.error) || typeof value.error.message !== "string") throw new NativeError("protocol", "Invalid camera error");
          fail(new NativeError(value.error.kind, value.error.message, "retryable" in value.error && value.error.retryable === true));
        } else if ((status === "ready" || (status === "active" && kind === "scanner" && "value" in value && typeof value.value === "object" && value.value !== null && "rawBytes" in value.value)) && "value" in value) {
          setState({ ...initial<T>(), status, ready: status === "active", value: decode(value.value) });
        } else if (status === "idle" || status === "opening" || status === "active") {
          setState({ ...initial<T>(), status, ready: status === "active" });
        } else throw new NativeError("protocol", "Invalid camera status");
      } catch (error) { fail(error); }
    });
    controller.current = attachment;
    void attachment.ready.then(() => {
      if (controller.current === attachment) setPreviewId(attachment.id);
    }, error => { if (controller.current === attachment) fail(error); });
    return () => {
      controller.current = null;
      void attachment.dispose().catch(error => console.error("Could not release camera", error));
    };
  }, [kind, decode, config]);
  const commands = useMemo(() => {
    const call = async (operation: string) => {
      if (!controller.current) throw new NativeError("unavailable", "Camera is not attached");
      await controller.current.call(operation);
    };
    return { capture: () => call("capture"), open: () => call("open"), retake: () => call("retake"), accept: () => call("use-photo") };
  }, []);
  return { kind, state, previewId, ...commands };
}
export function CameraPreview({ controller }: { controller: ReturnType<typeof useSession<CapturedPhoto>> | ReturnType<typeof useSession<CodeScan>> }) {
  const command = useAction((run: () => Promise<void>) => run());
  const { state } = controller;
  let content;
  if (state.reviewSource) {
    content = createElement(Stack, { align: "stretch", gap: 47 },
      createElement(Image, { src: state.reviewSource, width: 300, height: 340 }),
      createElement(Button, { disabled: command.status === "pending", onPress: () => command.run(controller.retake) }, "Retake"),
      createElement(Button, { disabled: state.saving || command.status === "pending", onPress: () => command.run(controller.accept) }, state.saving ? "Saving photo…" : "Use photo"));
  } else if (state.status === "ready" && state.value) {
    content = createElement(Stack, { align: "stretch", gap: 47 },
      "source" in state.value ? createElement(Image, { src: state.value.source, width: 300, height: 340 }) : createElement(Field, { label: "Code" }, state.value.text),
      createElement(Button, { disabled: command.status === "pending", onPress: () => command.run(controller.open) }, controller.kind === "photo" ? "Take another photo" : "Scan again"));
  } else if (state.error) {
    content = createElement(Stack, {}, createElement(Field, { label: "Camera" }, state.error.message),
      createElement(Button, { disabled: command.status === "pending", onPress: () => command.run(controller.open) }, "Try again"));
  } else if (controller.previewId !== null) {
    content = createElement("CameraPreview", { controller: controller.previewId, kind: controller.kind });
  } else content = createElement(Field, { label: "Camera" }, "Connecting...");
  return command.status === "error" ? createElement(Stack, {}, content, createElement(Field, { label: "Camera action" }, command.error.message)) : content;
}
