import { Barcode, type BarcodeFormat } from "@ink/barcode/generate";
import { Button, Screen, Text, useRouteParams } from "ink";

const examples: { format: BarcodeFormat; title: string; value: string }[] = [
  { format: "qr", title: "QR Code", value: "Hello World!" },
  { format: "aztec", title: "Aztec", value: "Hello World!" },
  { format: "data-matrix", title: "Data Matrix", value: "Hello World!" },
  { format: "pdf417", title: "PDF417", value: "Hello World!" },
  { format: "ean-13", title: "EAN-13", value: "5901234123457" },
  { format: "ean-8", title: "EAN-8", value: "96385074" },
  { format: "upc-a", title: "UPC-A", value: "036000291452" },
  { format: "upc-e", title: "UPC-E", value: "0123456" },
  { format: "code-39", title: "Code 39", value: "HELLO WORLD" },
  { format: "code-93", title: "Code 93", value: "HELLO WORLD" },
  { format: "code-128", title: "Code 128", value: "Hello World!" },
  { format: "itf", title: "ITF", value: "12345678" },
  { format: "codabar", title: "Codabar", value: "A123456A" },
];

export default function CodeGeneration() {
  return <Screen title="Code generation">
    {examples.map(example => <Button key={example.format}
      href={{ path: "/examples/code", params: { format: example.format } }}>{example.title}</Button>)}
  </Screen>;
}

export function GeneratedCode() {
  const { format } = useRouteParams<{ format: string }>();
  const example = examples.find(example => example.format === format);
  if (!example) return <Screen title="Code generation"><Text>Unknown code format.</Text></Screen>;
  return <Screen title={example.title} centered>
    <Barcode format={example.format} value={example.value} size={240} />
  </Screen>;
}
