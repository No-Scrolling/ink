import { useState } from "react";
import { Field, Row, Screen, Section, SettingsChoices, Text, TextInput, Toggle } from "ink";
import { arrowOutward, close } from "ink/icons";

// Every themed component in one screen, rendered by `ink theme check`.
export default function ThemeGallery() {
  const [choice, setChoice] = useState("one");
  return <Screen title="Theme" subtitle="Every themed component" note="Screen note">
    <Section title="Fields">
      <Field label="Label" onPress={() => {}}>Value</Field>
      <Field label="Long value" onPress={() => {}}>A value long enough to wrap under its label</Field>
    </Section>
    <Section title="Toggles">
      <Toggle label="On" value onChange={() => {}} />
      <Toggle label="Off" subtitle="With a subtitle" value={false} onChange={() => {}} />
      <Toggle label="Disabled" value disabled onChange={() => {}} />
    </Section>
    <Section title="Selection">
      <SettingsChoices value={choice} onChange={setChoice}
        options={[{ value: "one", label: "Selected" }, { value: "two", label: "Not selected" }]} />
    </Section>
    <Section title="Rows">
      <Row title="Link" subtitle="Subtitle" endIcon={arrowOutward} onPress={() => {}} />
      <Row title="Removable" endAction={{ icon: close, onPress: () => {} }} />
    </Section>
    <Text>Body text</Text>
    <Section title="Inputs">
      <TextInput label="Name" value="Alex" onChange={() => {}} action="done" />
    </Section>
  </Screen>;
}
