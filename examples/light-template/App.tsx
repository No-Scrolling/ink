import "@ink/light-sdk";
import { Navigator, Route, Tab, Tabs } from "ink";
import Confirm from "./screens/Confirm";
import Home from "./screens/Home";
import Modules from "./screens/Modules";
import Search from "./screens/Search";
import Settings from "./screens/Settings";
import Audio from "./screens/modules/Audio";
import Background from "./screens/modules/Background";
import Camera from "./screens/modules/Camera";
import Emoji from "./screens/modules/Emoji";
import LightSdk from "./screens/modules/LightSdk";
import Location from "./screens/modules/Location";
import Nfc from "./screens/modules/Nfc";
import Network from "./screens/modules/Network";
import Data from "./screens/modules/network/Data";
import Notifications from "./screens/modules/Notifications";
import Typography from "./screens/modules/Typography";
import LocalPlayback from "./screens/modules/audio/LocalPlayback";
import Microphone from "./screens/modules/audio/Microphone";
import Recording from "./screens/modules/audio/Recording";
import RemotePlayback from "./screens/modules/audio/RemotePlayback";
import Photo from "./screens/modules/camera/Photo";
import Scan from "./screens/modules/camera/Scan";
import Connection from "./screens/modules/light-sdk/Connection";
import Dialler from "./screens/modules/light-sdk/Dialler";
import Push from "./screens/modules/light-sdk/Push";
import Ringtone from "./screens/modules/light-sdk/Ringtone";
import RemoteImage from "./screens/modules/network/RemoteImage";
import Customise from "./screens/settings/Customise";
import CustomiseInterface from "./screens/settings/CustomiseInterface";
import DynamicUI from "./screens/settings/DynamicUI";
import TemperatureUnit from "./screens/settings/TemperatureUnit";
import TextInputExample from "./screens/settings/TextInputExample";

export default function LightTemplate() {
  return (
    <Navigator>
      <Route path="/">
        <Tabs>
          <Tab icon="home">
            <Home />
          </Tab>
          <Tab icon="search">
            <Search />
          </Tab>
          <Tab icon="widgets">
            <Modules />
          </Tab>
          <Tab icon="settings">
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
      <Route path="/modules/background">
        <Background />
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
      <Route path="/modules/camera">
        <Camera />
      </Route>
      <Route path="/modules/camera/photo">
        <Photo />
      </Route>
      <Route path="/modules/camera/scan">
        <Scan />
      </Route>
      <Route path="/modules/emoji">
        <Emoji />
      </Route>
      <Route path="/modules/light-sdk">
        <LightSdk />
      </Route>
      <Route path="/modules/light-sdk/connection">
        <Connection />
      </Route>
      <Route path="/modules/light-sdk/dialler">
        <Dialler />
      </Route>
      <Route path="/modules/light-sdk/ringtone">
        <Ringtone />
      </Route>
      <Route path="/modules/light-sdk/push">
        <Push />
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
      <Route path="/modules/network/data">
        <Data />
      </Route>
      <Route path="/modules/notifications">
        <Notifications />
      </Route>
      <Route path="/modules/typography">
        <Typography />
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
