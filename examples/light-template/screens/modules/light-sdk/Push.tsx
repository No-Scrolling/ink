import { lightPush } from "@ink/light-sdk";
import { Button, Screen, Stack, Text } from "ink";

export default function Push() {
  const push = lightPush();

  return (
    <Screen title="Push">
      <Text>Status: {push.status}</Text>
      {push.endpoint === "" ? null : <Text>Endpoint: {push.endpoint}</Text>}
      {push.status === "error" ? <Text>{push.error.message}</Text> : null}
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
          <Text>{message.title}</Text>
          <Text>{message.body}</Text>
          <Button onPress={() => push.dismiss(message.groupKey)}>Dismiss</Button>
        </Stack>
      ))}
    </Screen>
  );
}
