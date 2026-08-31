import { lightSdkPermission, lightSdkVersion } from "@ink/light-sdk";
import { Button, Field, Screen, match } from "ink";

export default function Connection() {
  const version = lightSdkVersion();
  const camera = lightSdkPermission("camera");

  return (
    <Screen title="Connection & Permissions">
      {match(version, {
        loading: () => <Field label="Light SDK">Connecting...</Field>,
        ready: (result) => <Field label="Light SDK version">{result.value}</Field>,
        error: (result) => <Field label="Light SDK">{result.error.message}</Field>,
      })}
      <Button onPress={() => version.reload()}>Refresh</Button>
      {match(camera, {
        loading: () => <Field label="Camera permission">Checking...</Field>,
        ready: (result) => <Field label="Camera permission">{result.value}</Field>,
        error: (result) => <Field label="Camera permission">{result.error.message}</Field>,
      })}
      <Button onPress={() => camera.request()}>Request Camera</Button>
    </Screen>
  );
}
