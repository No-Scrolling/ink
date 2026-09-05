import { Button, Screen } from "ink";

export default function Examples() {
  return (
    <Screen title="Examples">
      <Button href="/display/typography">Typography</Button>
      <Button href="/display/emoji">Emoji</Button>
      <Button href="/display/local-images">Local images</Button>
      <Button href="/settings/dynamic-ui">Dynamic UI</Button>
      <Button href="/settings/virtualised-list">Virtualised List</Button>
      <Button href="/confirm">Confirmation</Button>
      <Button href="/settings/screen-states">Screen States</Button>
    </Screen>
  );
}
