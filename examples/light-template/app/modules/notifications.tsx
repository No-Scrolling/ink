import { lightos } from "@ink/lightos";
import { useCallback, useEffect } from "react";
import { notifications, useNotificationTap } from "@ink/notifications";
import { Button, Field, Screen, Stack, useAction, useRouteParams } from "ink";

export default function Notifications() {
  const params = useRouteParams<{ source?: string }>();
  const permission = useAction(useCallback(() => lightos.getPermission("notifications"), []));
  const request = useAction(async () => {
    await lightos.requestPermission("notifications");
    permission.run();
  });
  const command = useAction((run: () => Promise<void>) => run());
  const tap = useNotificationTap();
  useEffect(() => permission.run(), [permission.run]);

  return (
    <Screen title="Notifications">
      <Field label="Permission">
        {permission.status === "success" ? permission.data
          : permission.status === "error" ? permission.error.message : "Checking..."}
      </Field>
      <Button onPress={() => request.run()}>Request Permission</Button>
      {request.status === "error" && <Field label="Permission error">{request.error.message}</Field>}
      <Button onPress={() => command.run(() => notifications.show({
        id: "example-reminder",
        title: "Ink reminder",
        body: "This notification was presented by Ink.",
        href: "/modules/notifications",
        data: "immediate",
      }))}>Show Now</Button>
      <Button onPress={() => command.run(() => notifications.schedule({
        id: "example-reminder",
        title: "Updated reminder",
        body: "The same ID atomically replaces the earlier reminder.",
        href: "/modules/notifications",
        data: "future",
        at: Date.now() + 15_000,
      }))}>Replace in 15 Seconds</Button>
      <Button onPress={() => command.run(() => notifications.cancel("example-reminder"))}>Cancel Reminder</Button>
      <Button onPress={() => command.run(lightos.requestExactPermission)}>Allow Exact Reminders</Button>
      <Button onPress={() => command.run(() => notifications.schedule({
        id: "example-reminder", title: "Exact reminder", body: "Open the reminder details.",
        at: Date.now() + 15_000, exact: true,
        href: { path: "/modules/notifications", params: { source: "exact-reminder" } },
        data: "exact",
      }))}>Exact in 15 Seconds</Button>
      {typeof params.source === "string" && <Field label="Route source">{params.source}</Field>}
      {command.status === "error" && <Field label="Error">{command.error.message}</Field>}
      {tap.state.status === "ready" ? (
        <Stack gap={16}>
          <Field label="Notification">{tap.state.value.id}</Field>
          <Field label="Data">{tap.state.value.data}</Field>
          <Button onPress={() => command.run(tap.consume)}>Consume Tap</Button>
        </Stack>
      ) : <Field label="Last tap">{tap.state.status === "error" ? tap.state.error.message : tap.state.status === "loading" ? "Loading..." : "None"}</Field>}
    </Screen>
  );
}
