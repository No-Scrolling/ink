import { Button, Screen } from "ink";

export default function Customise() {
  return (
    <Screen title="Customise">
      <Button href="/settings/customise-interface">Interface</Button>
      <Button href="/settings/option-example">Option Example</Button>
    </Screen>
  );
}
