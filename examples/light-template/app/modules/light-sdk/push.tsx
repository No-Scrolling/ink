import { setPushTask, usePush } from "@ink/lightos/push";
import { Button, Field, Screen, Stack, useAction, useSnapshot } from "ink";

import { processPush } from "../../../workers";
import { pushResult } from "../../../data/background";

export default function Push() {
  const push = usePush();
  const handled = useSnapshot(pushResult);
  const command = useAction((run: () => Promise<void>) => run());
  const disabled = !push.ready || command.status === "pending";

  return (
    <Screen title="Push">
      <Field label="Status">{push.ready ? push.state.status : "Connecting"}</Field>
      {push.state.endpoint !== "" && <Field label="Endpoint">{push.state.endpoint}</Field>}
      {push.state.error && <Field label="Error">{push.state.error.message}</Field>}
      <Button disabled={disabled} onPress={() => command.run(() => push.register("http://127.0.0.1:18080/v1/push/subscriptions"))}>Register Push</Button>
      <Button disabled={disabled} onPress={() => command.run(push.retry)}>Retry Registration</Button>
      <Button disabled={disabled} onPress={() => command.run(push.unregister)}>Unregister</Button>
      <Button disabled={disabled} onPress={() => command.run(push.clear)}>Clear Inbox</Button>
      <Button disabled={disabled} onPress={() => command.run(() => setPushTask(processPush))}>Enable Handler</Button>
      <Button disabled={disabled} onPress={() => command.run(() => setPushTask(null))}>Disable Handler</Button>
      {handled.status === "ready" && <Stack>
        <Field label="Background deliveries">{handled.data.deliveries}</Field>
        <Field label="Last handled">{handled.data.message}</Field>
      </Stack>}
      {command.status === "error" && <Field label="Command error">{command.error.message}</Field>}
      {push.state.messages.map(message => (
        <Stack key={message.id}>
          <Field label="Message">{message.title}: {message.body}</Field>
          <Button disabled={disabled} onPress={() => command.run(() => push.dismiss(message.groupKey))}>Dismiss</Button>
        </Stack>
      ))}
    </Screen>
  );
}
