import { Button, Screen, Stack, Text, Toggle, state } from "ink";

export default function DynamicUI() {
  const showApps = state(true);
  const apps = state(["Weather", "Passes"]);

  return (
    <Screen title="Dynamic UI">
      <Toggle
        label="Show Apps"
        value={showApps.value}
        onChange={() => showApps.set(!showApps.value)}
      />
      {showApps.value && (
        <Stack>
          {apps.value.length === 0 ? (
            <Stack gap={47}>
              <Text size={18}>No apps</Text>
              <Button onPress={() => apps.set(["Weather", "Passes"])}>Restore Apps</Button>
            </Stack>
          ) : (
            <Stack gap={47}>
              {apps.value.map((app) => (
                <Text>{app}</Text>
              ))}
              <Button onPress={() => apps.append("Beeper")}>Add Beeper</Button>
              <Button onPress={() => apps.clear()}>Clear Apps</Button>
            </Stack>
          )}
        </Stack>
      )}
      {!showApps.value && <Text size={18}>App list hidden</Text>}
    </Screen>
  );
}
