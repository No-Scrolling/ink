import { Button, Screen } from "ink";
import { barcodeExamples } from "../../data/barcodes";

export default function CodeGeneration() {
  return (
    <Screen title="Code generation">
      {barcodeExamples.map((example) => (
        <Button
          key={example.format}
          href={{ path: "/examples/code", params: { format: example.format } }}
        >
          {example.title}
        </Button>
      ))}
    </Screen>
  );
}
