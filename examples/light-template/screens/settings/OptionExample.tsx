import { Button, Screen, state } from "ink";

export default function OptionExample() {
  const option = state(0);

  return (
    <Screen title="Option Example">
      <Button onPress={() => option.set(0)}>Option 1</Button>
      <Button onPress={() => option.set(1)}>Option 2</Button>
      <Button onPress={() => option.set(2)}>Option 3</Button>
    </Screen>
  );
}
