import { useEffect, useState } from "react";
import { Screen, Stack, Text } from "ink";
import { onNativeMessage } from "ink/native";

declare const __inkPost: (source: string) => void;

export default function App() {
  const [cells, setCells] = useState(() => Array.from({ length: 500 }, () => 0));
  const [width, setWidth] = useState(12);
  useEffect(() => {
    const stop = onNativeMessage("benchmark", (message) => {
      if (message.command === "small")
        setCells((previous) => {
          const next = [...previous];
          next[0] = Number(message.value);
          return next;
        });
      if (message.command === "bulk" || message.command === "resize") {
        setCells(Array.from({ length: 500 }, () => Number(message.value)));
        if (message.command === "resize") setWidth(message.value === 1 ? 14 : 12);
      }
    });
    __inkPost(JSON.stringify({ type: "ready" }));
    return stop;
  }, []);
  return (
    <Screen title="Cells">
      <Stack gap={0}>
        {Array.from({ length: 25 }, (_, row) => (
          <Stack key={row} axis="horizontal">
            {Array.from({ length: 20 }, (_, column) => {
              const index = row * 20 + column;
              return (
                <Text key={index} width={width} size={4} maxLines={1}>
                  {String(index).padStart(3, "0") + ":" + cells[index]}
                </Text>
              );
            })}
          </Stack>
        ))}
      </Stack>
    </Screen>
  );
}
