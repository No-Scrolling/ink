import { createElement, useEffect, useMemo, useRef, useState } from "react";
import { Button, Field, Image, Stack, useAction } from "ink";
import { callNative, NativeError } from "ink/native";
import { attachNativeController } from "ink/native/controller";
import type { FileRef } from "@ink/files";

export type PermissionStatus = "granted" | "denied" | "blocked";
async function permission(operation: string): Promise<PermissionStatus> {
  const value = await callNative("permissions", operation, { permission: "camera" }, { timeoutMs: 120_000 });
  if (value !== "granted" && value !== "denied" && value !== "blocked") throw new NativeError("protocol", "Invalid camera permission result");
  return value;
}
export const camera = {
  getPermission: () => permission("status"),
  requestPermission: () => permission("request"),
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
  error: Error | null;
}
function initial<T>(): CameraState<T> { return { status: "idle", ready: false, value: null, reviewSource: null, error: null }; }
function decodePhoto(value: unknown): CapturedPhoto {
  if (typeof value !== "object" || value === null
    || !("source" in value) || typeof value.source !== "string"
    || !("width" in value) || typeof value.width !== "number" || !Number.isSafeInteger(value.width) || value.width <= 0
    || !("height" in value) || typeof value.height !== "number" || !Number.isSafeInteger(value.height) || value.height <= 0
    || !("mimeType" in value) || value.mimeType !== "image/jpeg"
    || !("capturedAtMs" in value) || typeof value.capturedAtMs !== "number" || !Number.isSafeInteger(value.capturedAtMs)) throw new NativeError("protocol", "Invalid captured photo");
  if (!("file" in value) || typeof value.file !== "object" || value.file === null
    || !("uri" in value.file) || typeof value.file.uri !== "string"
    || !("source" in value.file) || typeof value.file.source !== "string"
    || !("name" in value.file) || typeof value.file.name !== "string"
    || !("size" in value.file) || typeof value.file.size !== "number" || !Number.isSafeInteger(value.file.size) || value.file.size < 0
    || !("mimeType" in value.file) || value.file.mimeType !== "image/jpeg") throw new NativeError("protocol", "Invalid captured file");
  if (!("id" in value.file) || typeof value.file.id !== "string" || !("src" in value.file) || typeof value.file.src !== "string") throw new NativeError("protocol", "Invalid managed photo");
  const file: CapturedFile = { id: value.file.id, src: value.file.src, width: value.width, height: value.height, uri: value.file.uri, source: value.file.source, name: value.file.name, size: value.file.size, mimeType: value.file.mimeType };
  return { file, source: value.source, width: value.width, height: value.height, mimeType: value.mimeType, capturedAt: value.capturedAtMs };
}
function codeFormat(value: unknown): CodeFormat {
  const format = codeFormats.find(format => format === value);
  if (!format) throw new NativeError("protocol", "Invalid barcode format");
  return format;
}
function decodeCode(value: unknown): CodeScan {
  if (typeof value !== "object" || value === null || !("text" in value) || typeof value.text !== "string" || !("format" in value)) throw new NativeError("protocol", "Invalid scanned code");
  if (!("rawBytes" in value) || (value.rawBytes !== null && (!Array.isArray(value.rawBytes)
    || !value.rawBytes.every((byte: unknown) => typeof byte === "number" && Number.isInteger(byte) && byte >= 0 && byte <= 255)))) throw new NativeError("protocol", "Invalid barcode bytes");
  return { text: value.text, format: codeFormat(value.format), rawBytes: value.rawBytes === null ? null : new Uint8Array(value.rawBytes) };
}
function useSession<T>(kind: "photo" | "scanner", decode: (value: unknown) => T, config = "{}") {
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
          setState(previous => source ? { ...previous, status: "review", ready: false, reviewSource: source }
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
  return { kind, state, previewId, ...commands, requestPermission: camera.requestPermission };
}
export function useCamera({ facing = "back" }: { facing?: "back" | "front" } = {}) {
  return useSession("photo", decodePhoto, JSON.stringify({ facing }));
}
export function useCodeScanner({ formats = [...codeFormats], continuous = false, intervalMs = 1000, facing = "back" }: { formats?: readonly CodeFormat[]; continuous?: boolean; intervalMs?: number; facing?: "back" | "front" } = {}) {
  if (!Number.isSafeInteger(intervalMs) || intervalMs < 100 || intervalMs > 60_000) throw new RangeError("Scan interval must be between 100 and 60000 milliseconds");
  if (!formats.length) throw new TypeError("Choose at least one barcode format");
  return useSession("scanner", decodeCode, JSON.stringify({ formats: formats.map(codeFormat), continuous, intervalMs, facing }));
}
export function CameraPreview({ controller }: { controller: ReturnType<typeof useCamera> | ReturnType<typeof useCodeScanner> }) {
  const command = useAction((run: () => Promise<void>) => run());
  const { state } = controller;
  let content;
  if (state.reviewSource) {
    content = createElement(Stack, { align: "stretch", gap: 47 },
      createElement(Image, { src: state.reviewSource, width: 300, height: 340 }),
      createElement(Button, { disabled: command.status === "pending", onPress: () => command.run(controller.retake) }, "Retake"),
      createElement(Button, { disabled: command.status === "pending", onPress: () => command.run(controller.accept) }, "Use photo"));
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
