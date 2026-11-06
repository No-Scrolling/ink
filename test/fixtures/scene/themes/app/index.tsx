import { Canvas, CanvasText, Field, Rectangle, Screen, Section, Toggle } from "ink";

export default function Settings() {
  return <Screen title="Settings">
    <Canvas width={200} height={40}>
      <Rectangle x={0} y={0} width={40} height={40} fill="#ffffff" />
      <CanvasText x={60} y={0} width={140} height={40}>Canvas default</CanvasText>
    </Canvas>
    <Section title="Typing">
      <Field label="Spelling" href="/detail">British</Field>
      <Field label="Reader" href="/reader">Open</Field>
      <Field label="Keyboard" href="/rows">A value long enough that it cannot share a row with its label</Field>
    </Section>
    <Section title="Swiping">
      <Toggle label="Enabled toggle" value onChange={() => {}} />
      <Toggle label="Disabled toggle" value disabled onChange={() => {}} />
    </Section>
  </Screen>;
}
