import { CameraPreview, camera, useCamera } from "@ink/camera";
import { Button, Field, Screen, useAction, useRouteParams } from "ink";

export default function Photo() {
  const params = useRouteParams<{ facing?: string }>();
  const capture = useCamera({ facing: params.facing === "front" ? "front" : "back" });
  const read = useAction(async () => {
    const photo = capture.state.value;
    if (!photo) throw new Error("Take a photo first");
    const response = await fetch(photo.file.uri);
    if (!response.ok) throw new Error(`Photo could not be read (${response.status})`);
    return (await response.blob()).size;
  });
  const remove = useAction(async () => {
    const photo = capture.state.value;
    if (!photo) throw new Error("Take a photo first");
    await camera.removePhoto(photo.file);
    await capture.open();
  });

  return (
    <Screen title={params.facing === "front" ? "Front Camera" : "Photo"}>
      {capture.state.value && <>
        <Field label="Saved photo">{`${capture.state.value.file.size} bytes`}</Field>
        <Button disabled={read.status === "pending"} onPress={() => read.run()}>Read Photo</Button>
        {read.status === "success" && <Field label="Read">{`${read.data} bytes`}</Field>}
        {read.status === "error" && <Field label="Read">{read.error.message}</Field>}
        <Button disabled={remove.status === "pending"} onPress={() => remove.run()}>Delete Photo</Button>
        {remove.status === "error" && <Field label="Delete">{remove.error.message}</Field>}
      </>}
      <CameraPreview controller={capture} />
    </Screen>
  );
}
