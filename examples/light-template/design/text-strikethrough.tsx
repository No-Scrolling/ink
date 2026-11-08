import { Screen, Text } from "ink";

export default function Strikethrough({ decorated = true }: { decorated?: boolean }) {
  return (
    <Screen title="Strikethrough">
      <Text strikethrough={decorated}>
        Take the coastal path past the harbour and follow the signs to the lighthouse.
      </Text>
      <Text strikethrough={decorated} maxLines={1}>
        Take the coastal path past the harbour and follow the signs to the lighthouse.
      </Text>
      <Text strikethrough={decorated} align="center" size={24}>
        Completed
      </Text>
    </Screen>
  );
}
