import { Row, Screen } from "ink";
import artwork from "../../assets/images/wallsocket.jpg";
import spotify from "../../assets/icons/spotify.svg";
import { checkCircleFilled, download } from "ink/icons";

export default function Rows() {
  return <Screen title="Rows">
    <Row image={artwork} title="Wallsocket" subtitle="underscores" subtitleIcon={spotify} href="/display/local-images/wallsocket" />
    <Row title="Downloaded album" subtitle="Available offline" subtitleIcon={download} href="/actions" />
    <Row title="Feed entry" subtitle="Today at 10:30" href="/actions" />
    <Row title="A long title that stays on one line" titleMaxLines={1} subtitle="A longer subtitle that truncates rather than wrapping onto a second line." href="/actions" />
    <Row title="A long episode title that wraps onto another line" titleIcon={checkCircleFilled} subtitle="Finished" href="/actions" />
    <Row title="Title only" href="/actions" />
  </Screen>;
}
