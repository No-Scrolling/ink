import { Row, Screen } from "ink";
import { arrowOutward, close } from "ink/icons";

declare const __inkPost: (source: string) => void;
const post = (type: string, value: string) => __inkPost(JSON.stringify({ type, value }));

export default function Rows() {
  return <Screen title="Rows" note="Stored on this phone only">
    {Array.from({ length: 12 }, (_, index) => <Row
      key={index}
      title={`Item ${index}`}
      subtitle="Subtitle"
      onPress={() => post("selected", `row ${index}`)}
      endAction={index % 2 ? { icon: close, onPress: () => post("selected", `remove ${index}`) } : undefined}
      endIcon={index % 2 ? undefined : arrowOutward}
    />)}
  </Screen>;
}
