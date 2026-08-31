import { cameraPermission } from "@ink/camera";
import { Button, Screen, Text, match } from "ink";

export default function Camera() {
  const permission = cameraPermission();

  return (
    <Screen title="Camera">
      {match(permission, {
        loading: () => <Text>Checking camera permission...</Text>,
        ready: (result) => <Text>Permission: {result.value}</Text>,
        error: (result) => <Text>{result.error.message}</Text>,
      })}
      <Button onPress={() => permission.request()}>Request Camera</Button>
      <Button href="/modules/camera/photo">Photo</Button>
      <Button href="/modules/camera/scan">Scan Code</Button>
    </Screen>
  );
}
