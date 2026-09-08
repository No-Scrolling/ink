import { useEffect, useState } from "react";
import { Button, List, Screen, Text } from "ink";

const rows = Array.from({ length: 5000 }, (_, index) => ({ id: String(index + 1) }));

export default function VirtualisedList() {
  const [updates, setUpdates] = useState(0);
  const [running, setRunning] = useState(false);
  useEffect(() => {
    if (!running) return;
    const timer = setInterval(() => setUpdates(value => value + 1), 250);
    return () => clearInterval(timer);
  }, [running]);
  return <Screen title="Virtualised List">
    <Button onPress={() => setRunning(value => !value)}>{running ? "Stop updates" : "Start updates"}</Button>
    <Text>Updates: {updates}</Text>
    <List
      items={rows}
      keyExtractor={row => row.id}
      renderItem={row => <Text>Row {row.id}</Text>}
    />
  </Screen>;
}
