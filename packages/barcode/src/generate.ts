import { createElement } from "react";

export const supportedFormats = ["qr", "aztec", "data-matrix", "pdf417", "ean-13", "ean-8", "upc-a", "upc-e", "code-39", "code-93", "code-128", "itf", "codabar"] as const;
export type BarcodeFormat = typeof supportedFormats[number];

export function Barcode(props: {
  format: BarcodeFormat;
  value: string;
  size: number;
}) {
  if (!supportedFormats.includes(props.format)) throw new Error("Unsupported barcode format");
  if (!props.value.length) throw new Error("Barcode value must not be empty");
  if (!Number.isFinite(props.size) || props.size <= 0 || props.size > 1080) {
    throw new RangeError("Barcode size must be greater than zero and at most 1080");
  }
  return createElement("Barcode", props);
}
