import { lightSdkPermission, lightSdkVersion } from "@ink/light-sdk";
import { Button, Screen, Text, match } from "ink";

export default function Connection() {
  const version = lightSdkVersion();
  const camera = lightSdkPermission("camera");

  return (
    <Screen title="Connection & Permissions">
      {match(version, {
        loading: () => <Text>Connecting...</Text>,
        ready: (result) => <Text>Version: {result.value}</Text>,
        error: (result) => <Text>{result.error.message}</Text>,
      })}
      <Button onPress={() => version.reload()}>Refresh</Button>
      {match(camera, {
        loading: () => <Text>Checking Camera...</Text>,
        ready: (result) => <Text>Camera: {result.value}</Text>,
        error: (result) => <Text>{result.error.message}</Text>,
      })}
      <Button onPress={() => camera.request()}>Request Camera</Button>
    </Screen>
  );
}
