import { lightSdkPermission, lightSdkVersion } from "@ink/light-sdk";
import { Button, Screen, Text } from "ink";

export default function LightSdk() {
  const version = lightSdkVersion();
  const camera = lightSdkPermission("camera");

  return (
    <Screen title="Light SDK">
      {version.status === "loading" ? (
        <Text>Connecting...</Text>
      ) : version.status === "ready" ? (
        <Text>Version: {version.value}</Text>
      ) : (
        <Text>{version.error.message}</Text>
      )}
      <Button onPress={() => version.reload()}>Refresh</Button>
      {camera.status === "loading" ? (
        <Text>Checking Camera...</Text>
      ) : camera.status === "ready" ? (
        <Text>Camera: {camera.value}</Text>
      ) : (
        <Text>{camera.error.message}</Text>
      )}
      <Button onPress={() => camera.request()}>Request Camera</Button>
    </Screen>
  );
}
