import { Button, Screen, Text } from "ink";

export default function Confirm() {
  return (
    <Screen title="Example Confirm">
      <Text size={18}>This is an example confirmation screen. Are you sure you want to proceed?</Text>
      <Button>Yes</Button>
    </Screen>
  );
}
