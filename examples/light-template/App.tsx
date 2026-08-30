import {
  Button,
  Navigator,
  Route,
  Screen,
  Tab,
  Tabs,
  Text,
  TextInput,
  Toggle,
  state,
} from "ink";

export default function LightTemplate() {
  const tab = state(0);
  const query = state("");
  const name = state("");
  const invertColours = state(false);
  const option = state(0);

  return (
    <Navigator>
      <Route path="/">
        <Tabs value={tab.value}>
          <Tab icon="home" onPress={() => tab.set(0)}>
            <Screen title="Liked Songs">
              <Button>Test Button long one because I want to test a long button 1</Button>
              <Button>Test Button 2</Button>
              <Button>Test Button 3</Button>
              <Button>Test Button 4</Button>
              <Button>Test Button 5</Button>
              <Button>Test Button 6</Button>
              <Button>Test Button 7</Button>
              <Button>Test Button 8</Button>
              <Button>Test Button 9</Button>
              <Button>Test Button 10</Button>
            </Screen>
          </Tab>
          <Tab icon="search" onPress={() => tab.set(1)}>
            <Screen title="Search">
              <TextInput
                placeholder="Search..."
                value={query.value}
                onChange={(value) => query.set(value)}
                action="search"
              />
            </Screen>
          </Tab>
          <Tab icon="settings" onPress={() => tab.set(2)}>
            <Screen title="Settings">
              <Button href="/settings/customise">Customise</Button>
              <Button href="/settings/text-input">Text Input</Button>
              <Button href="/confirm">Example Confirm</Button>
            </Screen>
          </Tab>
        </Tabs>
      </Route>
      <Route path="/settings/customise">
        <Screen title="Customise">
          <Button href="/settings/customise-interface">Interface</Button>
          <Button href="/settings/option-example">Option Example</Button>
        </Screen>
      </Route>
      <Route path="/settings/customise-interface">
        <Screen title="Customise Interface">
          <Toggle
            label="Invert Colours"
            value={invertColours.value}
            onChange={() => invertColours.set(!invertColours.value)}
          />
        </Screen>
      </Route>
      <Route path="/settings/option-example">
        <Screen title="Option Example">
          <Button onPress={() => option.set(0)}>Option 1</Button>
          <Button onPress={() => option.set(1)}>Option 2</Button>
          <Button onPress={() => option.set(2)}>Option 3</Button>
        </Screen>
      </Route>
      <Route path="/settings/text-input">
        <Screen title="Text Input">
          <TextInput
            placeholder="Name..."
            value={name.value}
            onChange={(value) => name.set(value)}
            action="done"
          />
        </Screen>
      </Route>
      <Route path="/confirm">
        <Screen title="Example Confirm">
          <Text size={18}>This is an example confirmation screen. Are you sure you want to proceed?</Text>
          <Button>Yes</Button>
        </Screen>
      </Route>
    </Navigator>
  );
}
