import { MapView } from "@ink/maps";
import { Screen } from "ink";

const london = { latitude: 51.5074, longitude: -0.1278 };
export default function Maps() {
  return (
    <Screen title="Maps">
      <MapView
        initialCentre={london}
        initialZoom={12}
        markers={[{ id: "vehicle", ...london, label: "London" }]}
      />
    </Screen>
  );
}
