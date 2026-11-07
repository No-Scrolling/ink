import { Button, Screen, Text } from "ink";
import { close } from "ink/icons";

export default function HeaderSubtitle({ subtitle = "Choose how Atlas learns from your typing and keeps your personal dictionary up to date.\nYour words stay on this device." }: { subtitle?: string }) {
  return <Screen title="Learning" subtitle={subtitle} rightAction={{ icon: close, onPress: () => {} }}>
    <Text>Control which words Atlas remembers.</Text>
    <Button onPress={() => {}}>Manage learned words</Button>
  </Screen>;
}
