import { Button, Screen, Tab, Tabs, TextInput, state } from "ink";

export default function LightTemplate() {
  const tab = state(0);

  return (
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
          <TextInput placeholder="Search..." />
        </Screen>
      </Tab>
      <Tab icon="settings" onPress={() => tab.set(2)}>
        <Screen title="Settings">
          <Button>Customise</Button>
          <Button>Example Confirm</Button>
        </Screen>
      </Tab>
    </Tabs>
  );
}
