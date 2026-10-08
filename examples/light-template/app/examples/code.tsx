import { Barcode } from "@ink/barcode";
import { Screen, Text, useRouteParams } from "ink";
import { barcodeExamples } from "../../data/barcodes";

export default function GeneratedCode() {
  const { format } = useRouteParams();
  const example = barcodeExamples.find((example) => example.format === format);
  if (!example)
    return (
      <Screen title="Code generation">
        <Text>Unknown code format.</Text>
      </Screen>
    );
  return (
    <Screen title={example.title} centered>
      <Barcode format={example.format} value={example.value} size={240} />
    </Screen>
  );
}
