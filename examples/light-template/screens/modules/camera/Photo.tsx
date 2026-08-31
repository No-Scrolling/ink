import { CameraPreview, photoCapture } from "@ink/camera";
import { Screen } from "ink";

export default function Photo() {
  const capture = photoCapture();

  return (
    <Screen title="Photo">
      <CameraPreview session={capture} />
    </Screen>
  );
}
