import { lightos } from "@ink/lightos";
import { useCallback, useEffect } from "react";
import { Button, Field, Screen, useAction } from "ink";

export default function Camera() {
  const permission = useAction(useCallback(() => lightos.getPermission("camera"), []));
  const request = useAction(async () => {
    await lightos.requestPermission("camera");
    permission.run();
  });
  useEffect(() => permission.run(), [permission.run]);

  return (
    <Screen title="Camera">
      <Field label="Permission">
        {permission.status === "success" ? permission.data : permission.status === "error" ? permission.error.message : "Checking..."}
      </Field>
      <Button onPress={() => request.run()}>Request Camera</Button>
      {request.status === "error" && <Field label="Permission error">{request.error.message}</Field>}
      <Button href="/modules/camera/photo">Photo</Button>
      <Button href="/modules/camera/scan">Scan Code</Button>
      <Button href={{ path: "/modules/camera/photo", params: { facing: "front" } }}>Front Camera</Button>
      <Button href={{ path: "/modules/camera/scan", params: { continuous: "true" } }}>Continuous Scan</Button>
    </Screen>
  );
}
