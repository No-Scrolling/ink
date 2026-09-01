import { Navigator, Route } from "ink";
import Burst from "./screens/Burst";
import Emoji from "./screens/Emoji";
import Home from "./screens/Home";
import ListAppend from "./screens/ListAppend";
import Structure from "./screens/Structure";
import TextSameWidth from "./screens/TextSameWidth";
import TextWidthChange from "./screens/TextWidthChange";
import TogglePaint from "./screens/TogglePaint";

export default function UpdatesBenchmark() {
  return (
    <Navigator>
      <Route path="/">
        <Home />
      </Route>
      <Route path="/text-same-width">
        <TextSameWidth />
      </Route>
      <Route path="/text-width-change">
        <TextWidthChange />
      </Route>
      <Route path="/toggle">
        <TogglePaint />
      </Route>
      <Route path="/structure">
        <Structure />
      </Route>
      <Route path="/list-append">
        <ListAppend />
      </Route>
      <Route path="/burst">
        <Burst />
      </Route>
      <Route path="/emoji">
        <Emoji />
      </Route>
    </Navigator>
  );
}
