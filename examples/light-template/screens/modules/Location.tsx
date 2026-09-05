import { useCallback, useEffect, useRef, useState } from "react";
import { location, type LocationFix } from "@ink/location";
import { Button, Field, Screen, Stack, useAction } from "ink";

export default function Location() {
  const watchSession = useRef<AbortController | null>(null);
  const [watched, setWatched] = useState<LocationFix | null>(null);
  const [watchCount, setWatchCount] = useState(0);
  const watch = useAction(async () => {
    const controller = new AbortController();
    watchSession.current = controller;
    try {
      for await (const value of location.watch({ interval: 1000, signal: controller.signal })) {
        setWatched(value);
        setWatchCount(count => count + 1);
      }
    } catch (error) {
      if (!controller.signal.aborted) throw error;
    } finally {
      if (watchSession.current === controller) watchSession.current = null;
    }
  });
  const tracking = useAction(location.getTracking);
  const trackingPermission = useAction(location.requestTrackingPermission);
  const startTracking = useAction(async () => {
    await location.startTracking({ interval: 1000 });
    tracking.run();
  });
  const stopTracking = useAction(async () => { await location.stopTracking(); tracking.run(); });
  const permission = useAction(location.getPermission);
  const session = useRef<AbortController | null>(null);
  const current = useCallback(() => {
    const controller = new AbortController();
    session.current = controller;
    return location.current({ signal: controller.signal });
  }, []);
  const fix = useAction(current);
  const request = useAction(async () => {
    await location.requestPermission();
    permission.run();
  });
  useEffect(() => {
    permission.run();
    fix.run();
    tracking.run();
    return () => {
      watchSession.current?.abort();
      session.current?.abort();
      session.current = null;
    };
  }, [permission.run, fix.run, tracking.run]);

  return (
    <Screen title="Location">
      <Field label="Permission">
        {permission.status === "success" ? permission.data
          : permission.status === "error" ? permission.error.message : "Checking..."}
      </Field>
      <Button disabled={request.status === "pending"} onPress={() => request.run()}>Request Location</Button>
      {request.status === "error" && <Field label="Permission error">{request.error.message}</Field>}
      {fix.status === "success" ? (
        <Stack gap={16}>
          <Field label="Latitude">{fix.data.latitude}</Field>
          <Field label="Longitude">{fix.data.longitude}</Field>
          <Field label="Provider">{fix.data.provider}</Field>
        </Stack>
      ) : <Field label="Location">{fix.status === "error" ? fix.error.message : "Finding..."}</Field>}
      <Button disabled={fix.status === "pending"} onPress={() => fix.run()}>Refresh Location</Button>
      <Field label="Watch updates">{watchCount}</Field>
      {watched && <Field label="Watch coordinates">{watched.latitude.toFixed(5)}, {watched.longitude.toFixed(5)}</Field>}
      <Button onPress={() => watch.status === "pending" ? watchSession.current?.abort() : watch.run()}>
        {watch.status === "pending" ? "Stop watch" : "Start watch"}
      </Button>
      {watch.status === "error" && <Field label="Watch error">{watch.error.message}</Field>}
      <Field label="Background tracking">{tracking.status === "success" ? tracking.data.running ? "Running" : "Stopped" : "Checking..."}</Field>
      {tracking.status === "success" && tracking.data.fix && <Field label="Tracked coordinates">{tracking.data.fix.latitude.toFixed(5)}, {tracking.data.fix.longitude.toFixed(5)}</Field>}
      <Button disabled={trackingPermission.status === "pending"} onPress={() => trackingPermission.run()}>Tracking permissions</Button>
      {trackingPermission.status === "success" && <Field label="Tracking permission">{trackingPermission.data}</Field>}
      {trackingPermission.status === "error" && <Field label="Tracking permission error">{trackingPermission.error.message}</Field>}
      <Button disabled={startTracking.status === "pending"} onPress={() => startTracking.run()}>Start tracking</Button>
      <Button onPress={() => stopTracking.run()}>Stop tracking</Button>
      <Button onPress={() => tracking.run()}>Refresh tracking</Button>
      {startTracking.status === "error" && <Field label="Tracking error">{startTracking.error.message}</Field>}
    </Screen>
  );
}
