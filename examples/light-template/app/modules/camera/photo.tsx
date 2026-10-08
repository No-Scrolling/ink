import "@ink/network";
import { CameraPreview, useCamera } from "@ink/camera";
import { files } from "@ink/files";
import { Button, Field, Screen, useAction, useRouteParams } from "ink";

export default function Photo() {
  const params = useRouteParams();
  const capture = useCamera({ facing: params.facing === "front" ? "front" : "back" });
  const read = useAction(async () => {
    const photo = capture.state.value;
    if (!photo) throw new Error("Take a photo first");
    const response = await fetch(photo.file.src);
    if (!response.ok) throw new Error(`Photo could not be read (${response.status})`);
    return (await response.blob()).size;
  });
  const remove = useAction(async () => {
    const photo = capture.state.value;
    if (!photo) throw new Error("Take a photo first");
    await files.remove(photo.file.id);
    await capture.open();
  });

  return (
    <Screen title={params.facing === "front" ? "Front Camera" : "Photo"}>
      {capture.state.value && (
        <>
          <Field label="Saved photo">{`${capture.state.value.file.size} bytes`}</Field>
          <Button onPress={() => read.run()}>Read Photo</Button>
          {read.status === "success" && <Field label="Read">{`${read.data} bytes`}</Field>}
          {read.status === "error" && <Field label="Read">{read.error.message}</Field>}
          <Button onPress={() => remove.run()}>Delete Photo</Button>
          {remove.status === "error" && <Field label="Delete">{remove.error.message}</Field>}
        </>
      )}
      <CameraPreview controller={capture} />
    </Screen>
  );
}
