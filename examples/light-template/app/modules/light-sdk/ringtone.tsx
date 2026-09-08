import { installRingtone } from "@ink/lightos/ringtone";
import { Button, Field, Screen, useAction } from "ink";
import audio from "../../../assets/audio/cant_help.mp3";

export default function Ringtone() {
  const ringtone = useAction(() => installRingtone(audio));

  return (
    <Screen title="Ringtone">
      <Field label="Status">
        {ringtone.status === "idle" ? "Ready to install"
          : ringtone.status === "pending" ? "Installing..."
          : ringtone.status === "success" ? "Installed" : ringtone.error.message}
      </Field>
      <Button disabled={ringtone.status === "pending"} onPress={() => ringtone.run()}>Install Ringtone</Button>
    </Screen>
  );
}
