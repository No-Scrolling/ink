import { moreHoriz } from "ink/icons";
import { Button, navigate, Screen } from "ink";

export default function Examples() {
  return (
    <Screen title="Examples" rightAction={{ icon: moreHoriz, onPress: () => navigate("/actions") }}>
      <Button href="/examples/inputs">Inputs</Button>
      <Button href="/display/typography">Typography</Button>
      <Button href="/display/emoji">Emoji</Button>
      <Button href="/display/local-images">Local images</Button>
      <Button href="/settings/dynamic-ui">Dynamic UI</Button>
      <Button href="/settings/virtualised-list">Virtualised List</Button>
      <Button href="/settings/follow-items">Follow new items</Button>
      <Button href="/examples/conversation">Conversation</Button>
      <Button href="/examples/playing">Playing screen</Button>
      <Button href="/examples/rows">Rows</Button>
      <Button href="/examples/reordering">Reordering</Button>
      <Button href="/examples/pagination">Pagination</Button>
      <Button href="/examples/code-generation">Code generation</Button>
      <Button href="/confirm">Confirmation</Button>
      <Button href="/settings/screen-states">Screen States</Button>
    </Screen>
  );
}
