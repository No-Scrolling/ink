import { Screen, Text, useRouteParams } from "ink";

export default function ActionResult() {
  const { action } = useRouteParams<{ action: string }>();
  return <Screen title={`Action ${action}`}><Text>Action {action}</Text></Screen>;
}
