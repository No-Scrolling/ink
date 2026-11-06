import { useState } from "react";
import { Field, Screen, Section, Toggle } from "ink";
import { close } from "ink/icons";

export default function Settings() {
  const [autoCaps, setAutoCaps] = useState(true);

  return <Screen title="Settings" rightAction={{ icon: close, onPress: () => {} }}>
    <Section title="Setup">
      <Field label="Enable Atlas" href="/lift">Enabled</Field>
      <Field label="Choose Atlas" href="/lift">Enabled</Field>
    </Section>
    <Section title="Typing">
      <Field label="Spelling" href="/spelling">British</Field>
      <Toggle label="Auto-capitalisation" value={autoCaps} onChange={setAutoCaps} />
    </Section>
    <Section title="Swiping">
      <Field label="Continue after brief lift" href="/lift">120 ms</Field>
      <Field label="Calibration profile" href="/calibration">Atlas Calibration</Field>
      <Field label="Swipe vibration" href="/vibration">Default</Field>
    </Section>
    <Section title="Personal">
      <Field label="Learning" href="/learning">On</Field>
    </Section>
  </Screen>;
}
