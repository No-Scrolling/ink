import { Tab, Tabs } from "ink";
import Search from "./screens/Search";
import Settings from "./screens/Settings";
import Weather from "./screens/Weather";

export default function WeatherApp() {
  return (
    <Tabs>
      <Tab icon="place">
        <Weather />
      </Tab>
      <Tab icon="search">
        <Search />
      </Tab>
      <Tab icon="settings">
        <Settings />
      </Tab>
    </Tabs>
  );
}
