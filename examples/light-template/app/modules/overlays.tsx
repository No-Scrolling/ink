import { useState } from "react";
import { overlays, type OverlayPosition } from "@ink/overlays";
import { Button, Field, Screen, Text, useAction, useSnapshot } from "ink";
import { demoOverlay } from "../../overlays";

export default function Overlays() {
  const state = useSnapshot(overlays);
  const [position, setPosition] = useState<OverlayPosition>("bottom");
  const show = useAction(async () => {
    if (!(await overlays.isAllowed())) await overlays.requestPermission();
    if (!(await overlays.isAllowed())) throw new Error("Allow display over other apps, then try again.");
    await demoOverlay.show({ message: "Open another app to try it." }, { width: 360, height: 150, position, offset: 12 });
  });
  const update = useAction(async () => {
    await demoOverlay.update({ message: "Updated from the template." });
  });
  const move = useAction(async () => {
    const next = position === "bottom" ? "top" : "bottom";
    await demoOverlay.update({ message: "Tap the buttons in the overlay." }, { width: 360, height: 150, position: next, offset: 12 });
    setPosition(next);
  });
  const hide = useAction(async () => {
    await demoOverlay.hide();
  });
  const error = [show, update, move, hide].find(action => action.status === "error");

  return (
    <Screen title="Overlays">
      <Field label="Overlay">
        {state.status === "ready" ? (state.data.visible ? "Visible" : "Hidden") : state.status}
      </Field>
      <Button onPress={show.run}>Show overlay</Button>
      <Button onPress={update.run}>Update message</Button>
      <Button onPress={move.run}>Move to {position === "bottom" ? "top" : "bottom"}</Button>
      <Button onPress={hide.run}>Hide overlay</Button>
      {error?.status === "error" && <Text>{error.error.message}</Text>}
      {state.status === "error" && <Text>{state.error.message}</Text>}
    </Screen>
  );
}
