import { currentLocation, locationPermission } from "@ink/location";
import { Button, Field, Screen, Stack, match } from "ink";

export default function Location() {
  const permission = locationPermission();
  const location = currentLocation();

  return (
    <Screen title="Location">
      {match(permission, {
        loading: () => <Field label="Permission">Checking...</Field>,
        ready: (result) => <Field label="Permission">{result.value}</Field>,
        error: (result) => <Field label="Permission">{result.error.message}</Field>,
      })}
      <Button onPress={() => permission.request()}>Request Location</Button>
      {match(location, {
        loading: () => <Field label="Location">Finding...</Field>,
        ready: (result) => (
          <Stack gap={16}>
            <Field label="Latitude">{result.value.latitude}</Field>
            <Field label="Longitude">{result.value.longitude}</Field>
            <Field label="Provider">{result.value.provider}</Field>
          </Stack>
        ),
        error: (result) => <Field label="Location">{result.error.message}</Field>,
      })}
      <Button onPress={() => location.reload()}>Refresh Location</Button>
    </Screen>
  );
}
