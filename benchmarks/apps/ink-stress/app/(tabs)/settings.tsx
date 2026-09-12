import { Button, Screen, Text } from "ink";
import { useSettings } from "../../lib/state";

export default function Settings() {
  const { alternate, toggle } = useSettings();
  return <Screen title="Settings" centered>
    <Text>Dataset {alternate ? "B" : "A"}</Text>
    <Button onPress={toggle}>Change dataset</Button>
  </Screen>;
}
