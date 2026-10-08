import { PreferencesGate } from "../../components/PreferencesGate";
import { UnitChoice } from "../../components/UnitChoice";

export default function TimeFormatScreen() {
  return (
    <PreferencesGate title="Time Format">
      {(prefs) => (
        <UnitChoice
          title="Time Format"
          value={prefs.timeFormat}
          options={["24h", "12h"]}
          labels={{ "24h": "24 Hour", "12h": "12 Hour (AM/PM)" }}
          onSelect={(value) => ({ timeFormat: value })}
        />
      )}
    </PreferencesGate>
  );
}
