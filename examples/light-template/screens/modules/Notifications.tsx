import {
  localNotifications,
  notificationPermission,
  notificationTap,
} from "@ink/notifications";
import { Button, Field, Screen, Stack, match } from "ink";

export default function Notifications() {
  const permission = notificationPermission();
  const notifications = localNotifications();
  const tap = notificationTap();

  return (
    <Screen title="Notifications">
      {match(permission, {
        loading: () => <Field label="Permission">Checking...</Field>,
        ready: (result) => <Field label="Permission">{result.value}</Field>,
        error: (result) => <Field label="Permission">{result.error.message}</Field>,
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
        <Field label="Error">{notifications.error.message}</Field>
      ) : null}
      {match(tap, {
        empty: () => <Field label="Last tap">None</Field>,
        ready: (result) => (
          <Stack gap={16}>
            <Field label="Notification">{result.value.id}</Field>
            <Field label="Data">{result.value.data}</Field>
            <Button onPress={() => result.consume()}>Consume Tap</Button>
          </Stack>
        ),
      })}
    </Screen>
  );
}
