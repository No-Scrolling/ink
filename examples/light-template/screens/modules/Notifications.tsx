import {
  localNotifications,
  notificationPermission,
  notificationTap,
} from "@ink/notifications";
import { Button, Screen, Stack, Text, match } from "ink";

export default function Notifications() {
  const permission = notificationPermission();
  const notifications = localNotifications();
  const tap = notificationTap();

  return (
    <Screen title="Notifications">
      {match(permission, {
        loading: () => <Text>Checking permission…</Text>,
        ready: (result) => <Text>Permission: {result.value}</Text>,
        error: (result) => <Text>{result.error.message}</Text>,
      })}
      <Button onPress={() => permission.request()}>Request Permission</Button>
      <Button
        onPress={() =>
          notifications.schedule({
            id: "example-reminder",
            title: "Ink reminder",
            body: "This notification was presented by Ink.",
            href: "/modules/notifications",
            data: "immediate",
            delayMs: 0,
          })
        }
      >
        Show Now
      </Button>
      <Button
        onPress={() =>
          notifications.schedule({
            id: "example-reminder",
            title: "Updated reminder",
            body: "The same ID atomically replaces the earlier reminder.",
            href: "/modules/notifications",
            data: "future",
            delayMs: 15000,
          })
        }
      >
        Replace in 15 Seconds
      </Button>
      <Button onPress={() => notifications.cancel("example-reminder")}>
        Cancel Reminder
      </Button>
      {notifications.status === "error" ? (
        <Text>{notifications.error.message}</Text>
      ) : null}
      {match(tap, {
        empty: () => <Text>No notification tap</Text>,
        ready: (result) => (
          <Stack gap={16}>
            <Text>
              Tap: {result.value.id} ({result.value.data})
            </Text>
            <Button onPress={() => result.consume()}>Consume Tap</Button>
          </Stack>
        ),
      })}
    </Screen>
  );
}
