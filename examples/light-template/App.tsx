import Playing, { PlayingExample } from "./screens/Playing";
import Reordering from "./screens/Reordering";
import Rows from "./screens/Rows";
import Actions from "./screens/Actions";
import ActionResult from "./screens/ActionResult";
import Pagination from "./screens/Pagination";
import CodeGeneration, { GeneratedCode } from "./screens/CodeGeneration";
import icons from "./assets/navigation.ink-icons";
import { Navigator, Route, Tab, Tabs } from "ink";
import { AppearanceSettings } from "./data/appearance";
import Confirm from "./screens/Confirm";
import Examples from "./screens/Examples";
import Home from "./screens/Home";
import Modules from "./screens/Modules";
import Search from "./screens/Search";
import SearchResults from "./screens/SearchResults";
import Settings from "./screens/Settings";
import Emoji from "./screens/display/Emoji";
import Harbour from "./screens/display/Harbour";
import LocalImages from "./screens/display/LocalImages";
import Typography from "./screens/display/Typography";
import Wallsocket from "./screens/display/Wallsocket";
import Wills from "./screens/display/Wills";
import Audio from "./screens/modules/Audio";
import Background from "./screens/modules/Background";
import Camera from "./screens/modules/Camera";
import LightSdk from "./screens/modules/LightSdk";
import Location from "./screens/modules/Location";
import Nfc from "./screens/modules/Nfc";
import Network from "./screens/modules/Network";
import NetworkFeatures from "./screens/modules/network/Features";
import Data from "./screens/modules/network/Data";
import Notifications from "./screens/modules/Notifications";
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
import FollowItems from "./screens/settings/FollowItems";
import VirtualisedList from "./screens/settings/VirtualisedList";
import DynamicUI from "./screens/settings/DynamicUI";
import Selection from "./screens/settings/Selection";
import ScreenStates, { ScreenStateExample } from "./screens/settings/ScreenStates";

export default function LightTemplate() {
  return (
    <>
      <AppearanceSettings />
      <Navigator>
        <Route path="/examples/playing"><Playing /></Route>
        <Route path="/examples/playing/image"><PlayingExample image /></Route>
        <Route path="/examples/playing/no-image"><PlayingExample /></Route>
        <Route path="/">
          <Tabs>
            <Tab icon={icons.home}>
              <Home />
            </Tab>
            <Tab icon={icons.search}>
              <Search />
            </Tab>
            <Tab icon={icons.widgets}>
              <Modules />
            </Tab>
            <Tab icon={icons.palette}>
              <Examples />
            </Tab>
            <Tab icon={icons.settings}>
              <Settings />
            </Tab>
          </Tabs>
        </Route>
        <Route path="/actions"><Actions /></Route>
        <Route path="/examples/action"><ActionResult /></Route>
        <Route path="/examples/reordering"><Reordering /></Route>
        <Route path="/examples/rows"><Rows /></Route>
        <Route path="/examples/pagination"><Pagination /></Route>
        <Route path="/examples/code-generation"><CodeGeneration /></Route>
        <Route path="/examples/code"><GeneratedCode /></Route>
        <Route path="/search-results">
          <SearchResults />
        </Route>
        <Route path="/settings/selection">
          <Selection />
        </Route>
        <Route path="/settings/screen-states">
          <ScreenStates />
        </Route>
        <Route path="/settings/screen-states/loading">
          <ScreenStateExample result="ready" />
        </Route>
        <Route path="/settings/screen-states/error">
          <ScreenStateExample result="error" />
        </Route>
        <Route path="/settings/dynamic-ui">
          <DynamicUI />
        </Route>
        <Route path="/settings/follow-items"><FollowItems /></Route>
        <Route path="/settings/virtualised-list">
          <VirtualisedList />
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
        <Route path="/display/emoji">
          <Emoji />
        </Route>
        <Route path="/display/local-images">
          <LocalImages />
        </Route>
        <Route path="/display/local-images/wallsocket">
          <Wallsocket />
        </Route>
        <Route path="/display/local-images/wills">
          <Wills />
        </Route>
        <Route path="/display/local-images/harbour">
          <Harbour />
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
        <Route path="/modules/network/features"><NetworkFeatures /></Route>
        <Route path="/modules/network/data">
          <Data />
        </Route>
        <Route path="/modules/notifications">
          <Notifications />
        </Route>
        <Route path="/display/typography">
          <Typography />
        </Route>
        <Route path="/modules/network/remote-image">
          <RemoteImage />
        </Route>
        <Route path="/confirm">
          <Confirm />
        </Route>
      </Navigator>
    </>
  );
}
