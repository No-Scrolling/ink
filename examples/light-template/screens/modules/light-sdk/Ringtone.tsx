import { ringtoneInstaller } from "@ink/light-sdk";
import { Button, Field, Screen, match } from "ink";

export default function Ringtone() {
  const ringtone = ringtoneInstaller();

  return (
    <Screen title="Ringtone">
      {match(ringtone, {
        idle: () => <Field label="Status">Ready to install</Field>,
        installing: () => <Field label="Status">Installing...</Field>,
        installed: () => <Field label="Status">Installed</Field>,
        error: (result) => <Field label="Status">{result.error.message}</Field>,
      })}
      <Button
        onPress={() =>
          ringtone.set("./../audio/assets/cant_help.mp3", "ringtone")
        }
      >
        Install Ringtone
      </Button>
    </Screen>
  );
}
