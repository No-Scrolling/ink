import { currentLocation, locationPermission } from "@ink/location";
import { Button, Screen, Stack, Text, match } from "ink";

export default function Location() {
  const permission = locationPermission();
  const location = currentLocation();

  return (
    <Screen title="Location">
      {match(permission, {
        loading: () => <Text>Checking Location...</Text>,
        ready: (result) => <Text>Permission: {result.value}</Text>,
        error: (result) => <Text>{result.error.message}</Text>,
      })}
      <Button onPress={() => permission.request()}>Request Location</Button>
      {match(location, {
        loading: () => <Text>Finding Location...</Text>,
        ready: (result) => (
          <Stack gap={16}>
            <Text>Latitude: {result.value.latitude}</Text>
            <Text>Longitude: {result.value.longitude}</Text>
            <Text>Provider: {result.value.provider}</Text>
          </Stack>
        ),
        error: (result) => <Text>{result.error.message}</Text>,
      })}
      <Button onPress={() => location.reload()}>Refresh Location</Button>
    </Screen>
  );
}
