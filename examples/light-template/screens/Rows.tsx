import { Row, Screen } from "ink";
import artwork from "../assets/images/harbour.jpeg";

export default function Rows() {
  return <Screen title="Rows">
    <Row image={artwork} title="Album title" subtitle="Artist name" href="/display/local-images/harbour" />
    <Row title="Feed entry" subtitle="Today at 10:30" href="/actions" />
    <Row title="Chat name" subtitle="A longer message preview that wraps naturally onto another line." href="/actions" />
    <Row title="Title only" href="/actions" />
  </Screen>;
}
