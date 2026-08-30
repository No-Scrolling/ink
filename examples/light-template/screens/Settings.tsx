import { Button, Screen } from "ink";

export default function Settings() {
  return (
    <Screen title="Settings">
      <Button href="/settings/customise">Customise</Button>
      <Button href="/settings/text-input">Text Input</Button>
      <Button href="/settings/dynamic-ui">Dynamic UI</Button>
      <Button href="/settings/light-sdk">Light SDK</Button>
      <Button href="/settings/remote-image">Remote Image</Button>
      <Button href="/confirm">Example Confirm</Button>
    </Screen>
  );
}
