import {
  localNotifications,
  notificationPermission,
  notificationTap,
} from "@ink/notifications";
import { Button, Screen, Text } from "ink";

export default function Notifications() {
  const permission = notificationPermission();
  const notifications = localNotifications();
  const tap = notificationTap();

  return (
    <Screen title="Notifications">
      {permission.status === "ready" ? (
        <Text>Permission: {permission.value}</Text>
      ) : permission.status === "error" ? (
        <Text>{permission.error.message}</Text>
      ) : (
        <Text>Checking permission…</Text>
      )}
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
        <Text>{notifications.errorMessage}</Text>
      ) : null}
      {tap.status === "ready" ? (
        <Text>
          Tap: {tap.value.id} ({tap.value.data})
        </Text>
      ) : (
        <Text>No notification tap</Text>
      )}
      {tap.status === "ready" ? (
        <Button onPress={() => tap.consume()}>Consume Tap</Button>
      ) : null}
    </Screen>
  );
}
