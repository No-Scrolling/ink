import { Field, Screen } from "ink";

export default function Japanese() {
  return (
    <Screen title="日本語">
      <Field label="Hiragana">こんにちは。ありがとうございます。</Field>
      <Field label="Katakana">ポッドキャスト・ニュース・ミュージック</Field>
      <Field label="Kanji">東京都で音楽を聴く。今日は日曜日です。</Field>
      <Field label="Combining marks">が が　ぱ ぱ　ヴ ヴ</Field>
      <Field label="Mixed text">日本語と English、１２３と123。🎧 再生中 🇯🇵</Field>
      <Field label="Wrapping">
        「こんにちは、世界！」日本語の文章には、単語の間に空白がありません。
        長い文章を表示して、句読点や小さな文字を含む行の折り返しを確認します。
      </Field>
    </Screen>
  );
}
