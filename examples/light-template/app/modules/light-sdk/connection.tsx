import { useEffect } from "react";
import { lightos } from "@ink/lightos";
import { Button, Field, Screen, useAction } from "ink";

export default function Connection() {
  const version = useAction(lightos.getVersion);
  const preferences = useAction(lightos.getPreferences);
  const keyboard = useAction(lightos.getKeyboardOptions);
  const camera = useAction(() => lightos.getPermission("camera"));
  const request = useAction(async () => {
    await lightos.requestPermission("camera");
    camera.run();
  });
  useEffect(() => {
    version.run();
    camera.run();
    preferences.run();
    keyboard.run();
  }, [version.run, camera.run, preferences.run, keyboard.run]);

  return (
    <Screen title="Connection & Permissions">
      <Field label="Light SDK version">
        {version.status === "success"
          ? version.data
          : version.status === "error"
            ? version.error.message
            : "Connecting..."}
      </Field>
      <Button
        onPress={() => {
          if (version.status === "pending") return;
          version.run();
          preferences.run();
          keyboard.run();
        }}
      >
        Refresh
      </Button>
      <Field label="Host haptics">
        {preferences.status === "success"
          ? preferences.data.hapticsEnabled
            ? "On"
            : "Off"
          : preferences.status === "error"
            ? preferences.error.message
            : "Loading..."}
      </Field>
      <Field label="Keyboard voice input">
        {keyboard.status === "success"
          ? keyboard.data.displayVoice
            ? "Available"
            : "Hidden"
          : keyboard.status === "error"
            ? keyboard.error.message
            : "Loading..."}
      </Field>
      <Field label="Camera permission">
        {camera.status === "success"
          ? camera.data
          : camera.status === "error"
            ? camera.error.message
            : "Checking..."}
      </Field>
      <Button onPress={() => request.run()}>Request Camera</Button>
      {request.status === "error" && (
        <Field label="Permission error">{request.error.message}</Field>
      )}
    </Screen>
  );
}
