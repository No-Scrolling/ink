import { Tabs } from "ink";
import { partlyCloudyDay, place, settings } from "ink/icons";

export default function Layout() {
  return <Tabs>
    <Tabs.Screen name="index" icon={partlyCloudyDay} />
    <Tabs.Screen name="locations" icon={place} />
    <Tabs.Screen name="settings" icon={settings} />
  </Tabs>;
}
