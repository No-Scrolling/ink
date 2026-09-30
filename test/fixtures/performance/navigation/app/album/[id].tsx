import { List, Row, Screen, Text, useRouteParams } from "ink";

const tracks = Array.from({ length: 100 }, (_, index) => ({
  id: String(index),
  title: `Track ${index + 1}`,
  subtitle: "Málaga Ensemble · 3:20",
}));
export default function Album() {
  const { id } = useRouteParams("/album/[id]");
  return (
    <Screen title="Album" wide>
      <Text>Album {id}</Text>
      <List
        items={tracks}
        gap={8}
        keyExtractor={(track) => track.id}
        renderItem={(track) => (
          <Row title={track.title} titleMaxLines={1} subtitle={track.subtitle} />
        )}
      />
    </Screen>
  );
}
