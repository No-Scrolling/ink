import { Field, Screen } from "ink";

export default function Emoji() {
  return (
    <Screen title="Emoji">
      <Field label="Faces">😅 ☺️ 🙃 😍 😜 😂 😭 😎</Field>
      <Field label="People and skin tones">
        👋 👋🏽 🙌 👍 👎
      </Field>
      <Field label="Gestures">🤞 ✌️ 👌 🙏</Field>
      <Field label="Sequences and flags">
        👨‍👩‍👧‍👦 🧑‍💻 🏳️‍🌈 🇬🇧 1️⃣
      </Field>
      <Field label="Symbols">✨ 🔥 ❤️ 💔 🏆 🎯 👑 👀</Field>
    </Screen>
  );
}
