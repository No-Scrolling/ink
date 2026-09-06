import { Button, Screen } from "ink";

export default function Actions() {
  return <Screen title="Action Page">
    {[1, 2, 3].map(action => <Button key={action} href={{ path: "/examples/action", params: { action: String(action) } }}>Action {action}</Button>)}
  </Screen>;
}
