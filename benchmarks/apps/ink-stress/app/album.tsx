import { Image, Screen, Text, useRouteParams } from "ink";
import { artwork, items } from "../lib/catalogue";
import { useSettings } from "../lib/state";

export default function Album() {
  const { id } = useRouteParams();
  const item = items[Number(id) - 1];
  const { alternate } = useSettings();
  return <Screen title={item.title}>
    <Image src={artwork(item.id, alternate)} width={240} height={240} fit="cover" />
    <Text size={26}>Album details</Text>
    <Text size={18}>The same source image appears here at a larger size. Returning to the library must restore its rows and artwork.</Text>
  </Screen>;
}
