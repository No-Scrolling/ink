import { List, Row, Screen } from "ink";
import { download, musicNote, notifications } from "ink/icons";
import { artwork } from "../artwork";
import spotify from "../../../../examples/light-template/assets/icons/spotify.svg";

const variants = [
  { title: "A favourite album", subtitle: "A favourite artist", subtitleIcon: spotify },
  { title: "A live recording from the evening we spent at the old theatre", subtitle: "Available offline", subtitleIcon: download },
  { title: "A very long conversation name that should stay on one line", titleMaxLines: 1, subtitle: "Photo from Alex", subtitleIcon: notifications },
  { title: "Recently played", subtitle: "An especially long subtitle that should truncate beside its icon", subtitleIcon: musicNote },
  { title: "A reminder to bring headphones for the train journey tomorrow morning", subtitle: "Today at 14:35" },
  { title: "Favourites" },
  { title: "Downloaded album", subtitle: "Available offline", subtitleIcon: download },
  { title: "An album with a long title and no subtitle beneath it" },
];
const items = Array.from({ length: 500 }, (_, id) => {
  const variant = variants[id % variants.length];
  return { ...variant, id, image: artwork[id], title: `${id + 1}. ${variant.title}` };
});

export default function ScrollBenchmark() {
  return <Screen title="Scroll benchmark">
    <List items={items} gap={8} keyExtractor={item => String(item.id)}
      renderItem={({ id, ...row }) => <Row {...row} />} />
  </Screen>;
}
