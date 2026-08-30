import "@ink/light-sdk";
import { Navigator, Route, Tab, Tabs, state } from "ink";
import Confirm from "./screens/Confirm";
import Home from "./screens/Home";
import Search from "./screens/Search";
import Settings from "./screens/Settings";
import Customise from "./screens/settings/Customise";
import CustomiseInterface from "./screens/settings/CustomiseInterface";
import DynamicUI from "./screens/settings/DynamicUI";
import LightSdk from "./screens/settings/LightSdk";
import TemperatureUnit from "./screens/settings/TemperatureUnit";
import TextInputExample from "./screens/settings/TextInputExample";

export default function LightTemplate() {
  const tab = state(0);

  return (
    <Navigator>
      <Route path="/">
        <Tabs value={tab.value}>
          <Tab icon="home" onPress={() => tab.set(0)}>
            <Home />
          </Tab>
          <Tab icon="search" onPress={() => tab.set(1)}>
            <Search />
          </Tab>
          <Tab icon="settings" onPress={() => tab.set(2)}>
            <Settings />
          </Tab>
        </Tabs>
      </Route>
      <Route path="/settings/customise">
        <Customise />
      </Route>
      <Route path="/settings/customise-interface">
        <CustomiseInterface />
      </Route>
      <Route path="/settings/temperature-unit">
        <TemperatureUnit />
      </Route>
      <Route path="/settings/text-input">
        <TextInputExample />
      </Route>
      <Route path="/settings/dynamic-ui">
        <DynamicUI />
      </Route>
      <Route path="/settings/light-sdk">
        <LightSdk />
      </Route>
      <Route path="/confirm">
        <Confirm />
      </Route>
    </Navigator>
  );
}
