import { cameraPermission } from "@ink/camera";
import { Button, Screen, Text } from "ink";

export default function Camera() {
  const permission = cameraPermission();

  return (
    <Screen title="Camera">
      {permission.status === "ready" ? (
        <Text>Permission: {permission.value}</Text>
      ) : permission.status === "error" ? (
        <Text>{permission.error.message}</Text>
      ) : (
        <Text>Checking camera permission...</Text>
      )}
      <Button onPress={() => permission.request()}>Request Camera</Button>
      <Button href="/modules/camera/photo">Photo</Button>
      <Button href="/modules/camera/scan">Scan Code</Button>
    </Screen>
  );
}
