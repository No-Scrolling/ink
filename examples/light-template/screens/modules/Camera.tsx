import { cameraPermission } from "@ink/camera";
import { Button, Field, Screen, match } from "ink";

export default function Camera() {
  const permission = cameraPermission();

  return (
    <Screen title="Camera">
      {match(permission, {
        loading: () => <Field label="Permission">Checking...</Field>,
        ready: (result) => <Field label="Permission">{result.value}</Field>,
        error: (result) => <Field label="Permission">{result.error.message}</Field>,
      })}
      <Button onPress={() => permission.request()}>Request Camera</Button>
      <Button href="/modules/camera/photo">Photo</Button>
      <Button href="/modules/camera/scan">Scan Code</Button>
    </Screen>
  );
}
