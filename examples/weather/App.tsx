import { Tab, Tabs, state } from "ink";
import Search from "./screens/Search";
import Settings from "./screens/Settings";
import Weather from "./screens/Weather";

export default function WeatherApp() {
  const tab = state(0);

  return (
    <Tabs value={tab.value}>
      <Tab icon="place" onPress={() => tab.set(0)}>
        <Weather />
      </Tab>
      <Tab icon="search" onPress={() => tab.set(1)}>
        <Search />
      </Tab>
      <Tab icon="settings" onPress={() => tab.set(2)}>
        <Settings />
      </Tab>
    </Tabs>
  );
}
