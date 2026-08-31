import { openDialler } from "@ink/light-sdk";
import { Button, Screen, Text } from "ink";

export default function Dialler() {
  return (
    <Screen title="Dialler">
      <Text>Open the LightOS dialler with an example number. No call is placed.</Text>
      <Button onPress={() => openDialler("+15551234567")}>Open Dialler</Button>
    </Screen>
  );
}
