import { PitchIndicator } from "@ink/audio/microphone";
import { Screen, Stack, Text } from "ink";

export default function PitchPreview({
  note = "A4",
  cents = 0,
  frequency = 440,
}: {
  note?: string;
  cents?: number;
  frequency?: number;
}) {
  return (
    <Screen title="Tuner" centered>
      <Stack gap={14} align="center">
        <Text size={100} align="center">
          {note}
        </Text>
        <PitchIndicator cents={cents} />
        <Text size={20} align="center" tabularNumbers>
          {cents > 0 ? "+" : ""}
          {cents} cents · {frequency.toFixed(1)} Hz
        </Text>
      </Stack>
    </Screen>
  );
}
