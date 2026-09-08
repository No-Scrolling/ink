import { lightos } from "@ink/lightos";
import { Button, Field, Screen, Text, useAction } from "ink";

export default function Dialler() {
  const dialler = useAction(() => lightos.openDialler({ phoneNumber: "+15551234567" }));
  return (
    <Screen title="Dialler">
      <Text>Open the LightOS dialler with an example number. No call is placed.</Text>
      <Button disabled={dialler.status === "pending"} onPress={() => dialler.run()}>Open Dialler</Button>
      {dialler.status === "error" && <Field label="Error">{dialler.error.message}</Field>}
    </Screen>
  );
}
