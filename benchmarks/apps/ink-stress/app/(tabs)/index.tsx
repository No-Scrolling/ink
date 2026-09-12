import { List, LoadingState, Row, Screen, Text, useSnapshot } from "ink";
import { refresh } from "ink/icons";
import { artwork, items } from "../../lib/catalogue";
import { catalogue, useSettings } from "../../lib/state";

export default function Library() {
  const { alternate } = useSettings();
  const source = catalogue();
  const result = useSnapshot(source);
  const label = alternate ? "B" : "A";
  const status = result.status === "ready"
    ? `${result.refreshError ? "Retry" : result.refreshing ? "Loading" : "Ready"} ${result.data}`
    : "Loading";

  return <Screen title={`Library ${label}`} rightAction={{ icon: refresh, onPress: source.refresh }}
    header={<Text>{status}</Text>}>
    {result.status !== "ready" ? <LoadingState /> :
      <List key={label} items={items} gap={0} keyExtractor={item => String(item.id)}
        renderItem={item => <Row image={artwork(item.id, alternate)} title={item.title}
          subtitle={alternate ? "An alternative edition with a longer description" : "Local artwork and cached data"}
          href={{ path: "/album", params: { id: String(item.id) } }} />} />}
  </Screen>;
}
