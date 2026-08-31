import { CameraPreview, codeScanner } from "@ink/camera";
import { Screen } from "ink";

export default function Scan() {
  const scanner = codeScanner({ formats: ["qr", "ean-13", "code-128"] });

  return (
    <Screen title="Scan Code">
      <CameraPreview session={scanner} />
    </Screen>
  );
}
