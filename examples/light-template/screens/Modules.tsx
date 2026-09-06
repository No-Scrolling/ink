import { Button, Screen } from "ink";

export default function Modules() {
  return (
    <Screen title="Modules">
      <Button href="/modules/accounts">Accounts</Button>
      <Button href="/modules/audio">Audio</Button>
      <Button href="/modules/background">Background</Button>
      <Button href="/modules/camera">Camera</Button>
      <Button href="/modules/connectivity">Connectivity</Button>
      <Button href="/modules/database">Database</Button>
      <Button href="/modules/downloads">Downloads</Button>
      <Button href="/modules/external">External actions</Button>
      <Button href="/modules/files">Files</Button>
      <Button href="/modules/light-sdk">Light SDK</Button>
      <Button href="/modules/location">Location</Button>
      <Button href="/modules/maps">Maps</Button>
      <Button href="/modules/nfc">NFC</Button>
      <Button href="/modules/network">Network</Button>
      <Button href="/modules/notifications">Notifications</Button>
      <Button href="/modules/secure-store">Secure store</Button>
    </Screen>
  );
}
