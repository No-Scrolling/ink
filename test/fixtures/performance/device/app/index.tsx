import "@ink/network";
import { files } from "@ink/files";
import { useEffect, useState } from "react";
import { Image, List, Screen, Stack, Text, useRouteParams } from "ink";
import { callNative } from "ink/native";
import { images } from "../.ink/device-images";

const origin = "http://127.0.0.1:18766";
// Frozen 1080-pixel viewport: 2.55 physical pixels per Ink layout unit.
const artworkLayoutSize = 512 / 2.55;
type View = { mode: string; index: number; epoch: string; sequence: string };
let prepared: Promise<void> | undefined;

function prepare() {
  return prepared ??= callNative("device-benchmark", "prepare", { sources: images }).then(async result => {
    if (!result) throw Error("Managed image preparation returned no metadata");
    const file = await files.open("device-benchmark-0");
    if (file?.width !== 512 || file.height !== 512 || file.src !== "ink-file://device-benchmark-0") throw Error("Managed file did not reopen through the public SDK");
    console.log("INK_DEVICE_FILES_READY");
  });
}

export default function DevicePerformance() {
  const params = useRouteParams();
  const mode = String(params.mode ?? "blank"), index = Number(params.index ?? 0) % images.length;
  const epoch = String(params.epoch ?? "initial"), sequence = String(params.sequence ?? "startup");
  const [view, setView] = useState<View>();
  useEffect(() => {
    let active = true;
    void (mode === "file" ? prepare() : Promise.resolve()).then(async () => {
      const source = mode === "file" ? `ink-file://device-benchmark-${index}` : mode === "network"
        ? `${origin}/images/${epoch}/${index}.png` : images[index];
      await callNative("device-benchmark", "mark", { version: 1, mode, index, epoch, sequence, source });
      if (active) setView({ mode, index, epoch, sequence });
    }).catch(error => console.error(`INK_DEVICE_ERROR ${String(error)}`));
    return () => { active = false; };
  }, [mode, index, epoch, sequence]);
  useEffect(() => {
    if (view) console.log(`INK_DEVICE_READY ${JSON.stringify(view)}`);
  }, [view]);

  const src = view?.mode === "file" ? `ink-file://device-benchmark-${view.index}` : view?.mode === "network"
    ? `${origin}/images/${view.epoch}/${view.index}.png` : images[view?.index ?? 0];
  const items = images.map((_, item) => ({ id: String(item), index: item, source: `${origin}/images/${view?.epoch ?? "initial"}/${item}.png` }));
  return <Screen title="Device performance" wide>
    {!view ? <Text>Preparing encoded fixtures</Text> : view.mode === "gallery" ? (
      <List key={view.epoch} items={items} gap={8} keyExtractor={item => item.id}
        renderItem={item => <Stack gap={4}><Text size={12}>Artwork {item.index}</Text><Image src={item.source} width={artworkLayoutSize} height={artworkLayoutSize} fit="contain" /></Stack>} />
    ) : view.mode === "multilingual" ? <Stack gap={18}>
      <Text size={24} maxLines={1}>Ελληνικά · Καλημέρα κόσμε</Text>
      <Text size={24} maxLines={1}>Кириллица · Доброе утро</Text>
      <Text size={24} maxLines={1}>العربية · صباح الخير</Text>
      <Text size={24} maxLines={1}>中文 · 早上好世界</Text>
      <Text size={24} maxLines={1}>日本語 · おはよう世界</Text>
      <Text size={24} maxLines={1}>한국어 · 좋은 아침</Text>
    </Stack> : <Stack gap={16}>
      <Text size={22}>{view.mode} · {view.index} · {view.sequence}</Text>
      {view.mode !== "blank" && <Image src={src} width={artworkLayoutSize} height={artworkLayoutSize} fit="contain" />}
      <Text size={18}>Encoded PNG · 512 × 512 · Android image loader</Text>
    </Stack>}
  </Screen>;
}
