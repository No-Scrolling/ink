import { currentLocation, locationPermission } from "@ink/location";
import { Button, Screen, Stack, Text } from "ink";

export default function Location() {
  const permission = locationPermission();
  const location = currentLocation();

  return (
    <Screen title="Location">
      {permission.status === "ready" ? (
        <Text>Permission: {permission.value}</Text>
      ) : permission.status === "error" ? (
        <Text>{permission.error.message}</Text>
      ) : (
        <Text>Checking Location...</Text>
      )}
      <Button onPress={() => permission.request()}>Request Location</Button>
      {location.status === "ready" ? (
        <Stack gap={16}>
          <Text>Latitude: {location.value.latitude}</Text>
          <Text>Longitude: {location.value.longitude}</Text>
          <Text>Provider: {location.value.provider}</Text>
        </Stack>
      ) : location.status === "error" ? (
        <Text>{location.error.message}</Text>
      ) : (
        <Text>Finding Location...</Text>
      )}
      <Button onPress={() => location.reload()}>Refresh Location</Button>
    </Screen>
  );
}
