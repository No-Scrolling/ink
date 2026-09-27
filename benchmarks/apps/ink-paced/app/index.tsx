import { Activity, useEffect, useState } from "react";
import { Screen, Stack, Text } from "ink";
import { refresh } from "ink/icons";

export default function Updates() {
  const [count, setCount] = useState(500);
  const [tick, setTick] = useState(0);
  const [reverse, setReverse] = useState(false);
  const [hidden, setHidden] = useState(false);
  const [resize, setResize] = useState(true);
  const [running, setRunning] = useState(false);
  useEffect(() => {
    if (!running) return;
    const start = performance.now();
    let frame = 0;
    let timer: ReturnType<typeof setTimeout>;
    function advance() {
      setTick(value => value + 1);
      if (++frame === 180) { setRunning(false); return; }
      timer = setTimeout(advance, Math.max(0, Math.ceil(start + (frame + 1) * 1000 / 60 - performance.now())));
    }
    timer = setTimeout(advance, 17);
    return () => clearTimeout(timer);
  }, [running]);
  const ids = Array.from({ length: count }, (_, id) => id);
  if (reverse) ids.reverse();
  const rows = Array.from({ length: Math.ceil(count / 10) }, (_, row) => ids.slice(row * 10, row * 10 + 10));

  return <Screen title="Update benchmark" rightAction={{ icon: refresh, onPress: () => setRunning(value => !value) }}
    header={<Stack gap={8}>
      <Stack axis="horizontal" gap={16}>
        {[1, 10, 100, 500].map(value => <Text key={value} size={18} width={50} onPress={() => { setCount(value); setTick(0); }}>{value}</Text>)}
      </Stack>
      <Stack axis="horizontal" gap={16}>
        <Text size={14} width={85} onPress={() => setReverse(value => !value)}>Reverse</Text>
        <Text size={14} width={85} onPress={() => setHidden(value => !value)}>{hidden ? "Show" : "Hide"}</Text>
        <Text size={14} width={85} onPress={() => setResize(value => !value)}>{resize ? "Resize on" : "Resize off"}</Text>
      </Stack>
      <Text size={14}>{`${count} cells · ${tick} updates`}</Text>
    </Stack>}>
    <Activity mode={hidden ? "hidden" : "visible"}>
      <Stack gap={4}>
        {rows.map((row, index) => <Stack key={index} axis="horizontal" gap={resize ? tick % 2 : 0}>
          {row.map(id => <Text key={id} size={8} width={30} maxLines={1} tabularNumbers>{`${String(id).padStart(3, "0")}:${(id + tick) % 100}`}</Text>)}
        </Stack>)}
      </Stack>
    </Activity>
  </Screen>;
}
