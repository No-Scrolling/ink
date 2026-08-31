import { Button, Screen } from "ink";

export default function LightSdk() {
  return (
    <Screen title="Light SDK">
      <Button href="/modules/light-sdk/connection">Connection & Permissions</Button>
      <Button href="/modules/light-sdk/dialler">Dialler</Button>
      <Button href="/modules/light-sdk/ringtone">Ringtone</Button>
      <Button href="/modules/light-sdk/push">Push</Button>
    </Screen>
  );
}
