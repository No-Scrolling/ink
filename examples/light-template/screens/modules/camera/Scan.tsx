import { CameraPreview, useCodeScanner } from "@ink/barcode/scan";
import { Field, Screen, useRouteParams } from "ink";

export default function Scan() {
  const params = useRouteParams<{ continuous?: string }>();
  const continuous = params.continuous === "true";
  const scanner = useCodeScanner({ formats: ["qr", "ean-13", "code-128"], continuous });

  return (
    <Screen title={continuous ? "Continuous Scan" : "Scan Code"}>
      {continuous && <Field label="Latest code">{scanner.state.value?.text ?? "Scanning..."}</Field>}
      {scanner.state.value && <Field label="Decoder bytes">
        {scanner.state.value.rawBytes === null ? "Unavailable for this code" : `${scanner.state.value.rawBytes.length} bytes`}
      </Field>}
      <CameraPreview controller={scanner} />
    </Screen>
  );
}
