import * as v from "valibot";
import { Screen, Text, useRouteParams } from "ink";

export default function ActionResult() {
  const { action } = useRouteParams(
    v.parser(v.object({ action: v.fallback(v.string(), "Unknown") })),
  );
  return (
    <Screen title={`Action ${action}`}>
      <Text>Action {action}</Text>
    </Screen>
  );
}
