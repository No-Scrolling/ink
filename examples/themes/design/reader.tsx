import { Screen, Text } from "ink";

export default function Reader() {
  return <Screen title="Learning notes" subtitle="How Atlas learns your words" theme="reader">
    <Text>Atlas learns the words you choose from the suggestion strip and the words you type often.</Text>
    <Text size={22}>Learned words stay on this phone. Remove one from Learned words at any time.</Text>
  </Screen>;
}
