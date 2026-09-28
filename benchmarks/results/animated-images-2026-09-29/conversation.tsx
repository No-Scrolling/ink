import { ConversationScreen } from "ink";
import finite from "../finite.gif";
const messages = [{ id: "gif", timestamp: 0, outgoing: false, image: { src: finite, width: 180, height: 135 } }];
export default function Conversation() {
  return <ConversationScreen title="Looping GIF conversation" messages={messages} draft="" onDraftChange={() => {}} onSend={() => {}} />;
}
