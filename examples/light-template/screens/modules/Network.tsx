import { Button, Screen } from "ink";

export default function Network() {
  return (
    <Screen title="Network">
      <Button href={{ path: "/modules/network/data", params: { label: "Data Resources" } }}>
        Data Resources
      </Button>
      <Button href="/modules/network/features">Network Features</Button>
      <Button href="/modules/network/remote-image">Remote Image</Button>
    </Screen>
  );
}
