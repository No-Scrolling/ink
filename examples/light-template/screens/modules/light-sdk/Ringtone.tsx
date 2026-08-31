import { ringtoneInstaller } from "@ink/light-sdk";
import { Button, Screen, Text } from "ink";

export default function Ringtone() {
  const ringtone = ringtoneInstaller();

  return (
    <Screen title="Ringtone">
      <Text>Status: {ringtone.status}</Text>
      {ringtone.status === "error" ? (
        <Text>{ringtone.errorMessage}</Text>
      ) : null}
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
