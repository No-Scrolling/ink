import { useEffect } from "react";
import { Canvas, CanvasText, Image, List, Rectangle, Row, Screen, Stack, Text, useRouteParams } from "ink";
import pattern from "../assets/pattern.png";
import { artwork } from "../.ink/artwork";

const items = Array.from({ length: 32 }, (_, index) => ({
  id: String(index + 1),
  label: `Row ${String(index + 1).padStart(2, "0")} · ${index % 2 ? "Béatrice" : "Málaga"}`,
}));
const titles = ["Harbour Lights", "An evening recording from the old theatre beside the river",
  "Málaga at Dusk", "Northern Lines", "Quiet Rooms", "Zürich Sessions", "The Long Way Home", "First Light"];
const artists = ["Béatrice", "Málaga Ensemble", "Zürich Quartet", "Northbound"];
const albums = artwork.map((image, index) => ({ id: String(index + 1), image,
  title: `${String(index + 1).padStart(2, "0")} · ${titles[index % titles.length]}`,
  subtitle: artists[index % artists.length] }));

export default function VisualFixture() {
  const requested = useRouteParams<{ fixtureState?: string }>().fixtureState;
  const state = requested === "list" || requested === "rows" ? requested : "graphics";
  useEffect(() => {
    console.log(`INK_VISUAL_STATE ${JSON.stringify({ version: 1, state })}`);
  }, [state]);
  return <Screen waitForImages wide={state === "rows"} title={state === "graphics" ? "Graphics" : state === "rows" ? "Rows" : "List"}>
    {state === "graphics" ? <Stack gap={16}>
      <Text size={18}>Clipped canvas</Text>
      <Canvas width={300} height={90}>
        <Rectangle x={0} y={0} width={75} height={40} fill="#777777" />
        <Rectangle x={245} y={60} width={100} height={60} fill="#BBBBBB" />
        <Rectangle x={1} y={1} width={298} height={88} stroke="#FFFFFF" strokeWidth={2} />
        <Rectangle x={227} y={20} width={1} height={60} fill="#777777" />
        <CanvasText x={85} y={25} width={142} height={50} size={36}>CLIPPED</CanvasText>
      </Canvas>
      <Text size={18} width={260} maxLines={2}>Málaga, Béatrice and Zürich wrap into two lines; the rest of this sentence is truncated.</Text>
      <Stack axis="horizontal" gap={20}>
        <Stack gap={8}>
          <Text size={16}>Contain</Text>
          <Image src={pattern} width={125} height={90} fit="contain" />
        </Stack>
        <Stack gap={8}>
          <Text size={16}>Cover</Text>
          <Image src={pattern} width={125} height={90} fit="cover" />
        </Stack>
      </Stack>
    </Stack> : state === "rows" ? <List key="rows" items={albums} gap={8} keyExtractor={item => item.id}
      renderItem={item => <Row title={item.title} titleMaxLines={1} subtitle={item.subtitle} image={item.image} />} />
      : <List key="list" items={items} gap={18} keyExtractor={item => item.id}
      renderItem={item => <Text size={24}>{item.label}</Text>} />}
  </Screen>;
}
