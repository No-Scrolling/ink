import { Button, Screen } from "ink";

export default function Modules() {
  return (
    <Screen title="Modules">
      <Button href="/modules/audio">Audio</Button>
      <Button href="/modules/background">Background</Button>
      <Button href="/modules/camera">Camera</Button>
      <Button href="/modules/light-sdk">Light SDK</Button>
      <Button href="/modules/location">Location</Button>
      <Button href="/modules/nfc">NFC</Button>
      <Button href="/modules/network">Network</Button>
      <Button href="/modules/notifications">Notifications</Button>
    </Screen>
  );
}
