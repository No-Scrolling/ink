import { List, Row, Screen } from "ink";
import artwork from "../../../examples/light-template/assets/images/wallsocket.jpg";

const items = Array.from({ length: 500 }, (_, id) => ({ id, title: `Album ${id + 1}` }));

export default function ScrollBenchmark() {
  return <Screen title="Scroll benchmark">
    <List items={items} keyExtractor={item => String(item.id)}
      renderItem={item => <Row image={artwork} title={item.title} subtitle="Artist • A thumbnail and two lines of text" />} />
  </Screen>;
}
