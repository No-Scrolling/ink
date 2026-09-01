import { Screen, Stack, Text } from "ink";

export default function Typography() {
  return (
    <Screen title="Typography and Truncation">
      <Stack gap={0}>
        <Text size={20}>Wrapping</Text>
        <Text size={24}>
          Ink wraps text automatically at natural Unicode line breaks, including text with emoji 👋🏽.
        </Text>
      </Stack>
      <Stack gap={0}>
        <Text size={20}>Justification</Text>
        <Text size={24} align="justify">
          Justified text distributes each wrapped line across the available width while keeping its
          final line aligned to the start.
        </Text>
      </Stack>
      <Stack gap={0}>
        <Text size={20}>Truncation</Text>
        <Text size={24} maxLines={2}>
          Set a maximum number of lines to wrap longer text and truncate anything that remains with
          an ellipsis.
        </Text>
      </Stack>
    </Screen>
  );
}
