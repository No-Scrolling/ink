import { useState } from "react";
import { Button, Image, Screen, Stack, Text } from "ink";
import loop from "../loop.gif";
import finite from "../finite.gif";
export default function GifDemo() {
  const [viewer, setViewer] = useState(false);
  const [hidden, setHidden] = useState(false);
  return <Screen title={viewer ? "GIF viewer" : "GIF thumbnails"}>
    <Button onPress={() => setViewer(!viewer)}>{viewer ? "Thumbnails" : "Open viewer"}</Button>
    <Button onPress={() => setHidden(!hidden)}>{hidden ? "Show images" : "Hide images"}</Button>
    {!hidden && <Stack gap={16}>
      <Text>Looping, transparent</Text>
      <Image src={loop} width={viewer ? 360 : 160} height={viewer ? 270 : 120} fit="contain" zoomable={viewer} />
      <Text>Plays twice then stops</Text>
      <Image src={finite} width={160} height={120} fit="contain" />
    </Stack>}
  </Screen>;
}
