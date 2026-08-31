import { ringtoneInstaller } from "@ink/light-sdk";
import { Button, Screen, Text, match } from "ink";

export default function Ringtone() {
  const ringtone = ringtoneInstaller();

  return (
    <Screen title="Ringtone">
      {match(ringtone, {
        idle: () => <Text>Ready to install</Text>,
        installing: () => <Text>Installing...</Text>,
        installed: () => <Text>Installed</Text>,
        error: (result) => <Text>{result.errorMessage}</Text>,
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
