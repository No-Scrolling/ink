import { useState } from "react";
import { Field, Screen, Section, Toggle } from "ink";

const words = ["Atlas", "Chicken", "Dave", "Gerome", "Pingu", "Titan"];

export default function LearnedWords() {
  const [learning, setLearning] = useState(true);

  return <Screen title="Learned words">
    <Toggle label="Learn my words" value={learning} onChange={setLearning} />
    <Section title="Backup">
      <Field label="Export words" onPress={() => {}}>{""}</Field>
      <Field label="Import words" onPress={() => {}}>{""}</Field>
      <Field label="Reset learned words" onPress={() => {}}>{""}</Field>
    </Section>
    <Section title="Words">
      {words.map(word => <Field key={word} label={word} onPress={() => {}}>{""}</Field>)}
    </Section>
  </Screen>;
}
