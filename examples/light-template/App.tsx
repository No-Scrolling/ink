import "@ink/light-sdk";
import { Navigator, Route, Tab, Tabs, state } from "ink";
import Confirm from "./screens/Confirm";
import Home from "./screens/Home";
import Modules from "./screens/Modules";
import Search from "./screens/Search";
import Settings from "./screens/Settings";
import Audio from "./screens/modules/Audio";
import LightSdk from "./screens/modules/LightSdk";
import Location from "./screens/modules/Location";
import Nfc from "./screens/modules/Nfc";
import Network from "./screens/modules/Network";
import LocalPlayback from "./screens/modules/audio/LocalPlayback";
import Microphone from "./screens/modules/audio/Microphone";
import Recording from "./screens/modules/audio/Recording";
import RemotePlayback from "./screens/modules/audio/RemotePlayback";
import RemoteImage from "./screens/modules/network/RemoteImage";
import Customise from "./screens/settings/Customise";
import CustomiseInterface from "./screens/settings/CustomiseInterface";
import DynamicUI from "./screens/settings/DynamicUI";
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
          <Tab icon="widgets" onPress={() => tab.set(2)}>
            <Modules />
          </Tab>
          <Tab icon="settings" onPress={() => tab.set(3)}>
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
      <Route path="/modules/audio">
        <Audio />
      </Route>
      <Route path="/modules/audio/local-playback">
        <LocalPlayback />
      </Route>
      <Route path="/modules/audio/remote-playback">
        <RemotePlayback />
      </Route>
      <Route path="/modules/audio/recording">
        <Recording />
      </Route>
      <Route path="/modules/audio/microphone">
        <Microphone />
      </Route>
      <Route path="/modules/light-sdk">
        <LightSdk />
      </Route>
      <Route path="/modules/location">
        <Location />
      </Route>
      <Route path="/modules/nfc">
        <Nfc />
      </Route>
      <Route path="/modules/network">
        <Network />
      </Route>
      <Route path="/modules/network/remote-image">
        <RemoteImage />
      </Route>
      <Route path="/confirm">
        <Confirm />
      </Route>
    </Navigator>
  );
}
