import { lightPush } from "@ink/light-sdk";
import { Button, Field, Screen, Stack } from "ink";

export default function Push() {
  const push = lightPush();

  return (
    <Screen title="Push">
      <Field label="Status">{push.status}</Field>
      {push.endpoint === "" ? null : (
        <Field label="Endpoint">{push.endpoint}</Field>
      )}
      {push.status === "error" ? (
        <Field label="Error">{push.error.message}</Field>
      ) : null}
      <Button
        onPress={() =>
          push.register("http://127.0.0.1:18080/v1/push/subscriptions")
        }
      >
        Register Push
      </Button>
      <Button onPress={() => push.retry()}>Retry Registration</Button>
      <Button onPress={() => push.unregister()}>Unregister</Button>
      <Button onPress={() => push.clear()}>Clear Inbox</Button>
      {push.messages.map((message) => (
        <Stack>
          <Field label="Message">
            {message.title}: {message.body}
          </Field>
          <Button onPress={() => push.dismiss(message.groupKey)}>Dismiss</Button>
        </Stack>
      ))}
    </Screen>
  );
}
