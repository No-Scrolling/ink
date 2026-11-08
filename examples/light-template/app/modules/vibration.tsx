import { vibrate, stopVibrating } from "@ink/vibration";
import { Button, Screen, Text, useAction } from "ink";

const pattern = [
  { ms: 300, strength: 1 },
  { ms: 150, strength: 0 },
  { ms: 300, strength: 0.5 },
];

export default function Vibration() {
  const action = useAction((operation: "touch" | "soft" | "pattern" | "alarm" | "stop") => {
    switch (operation) {
      case "touch":
        return vibrate(200);
      case "soft":
        return vibrate(200, { strength: 0.4 });
      case "pattern":
        return vibrate(pattern);
      case "alarm":
        return vibrate(pattern, { alarm: true });
      case "stop":
        return stopVibrating();
    }
  });
  return (
    <Screen title="Vibration">
      <Button onPress={() => action.run("touch")}>Vibrate</Button>
      <Button onPress={() => action.run("soft")}>Vibrate softly</Button>
      <Button onPress={() => action.run("pattern")}>Play pattern</Button>
      <Button onPress={() => action.run("alarm")}>Play alarm</Button>
      <Button onPress={() => action.run("stop")}>Stop vibrating</Button>
      {action.status === "error" && <Text>{action.error.message}</Text>}
    </Screen>
  );
}
