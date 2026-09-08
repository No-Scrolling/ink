import { MediaPicker } from "@ink/files/media";
import { back } from "ink";
import { attachments } from "../../../data/attachments";

export default function MediaLibrary() {
  return <MediaPicker onSelect={async selected => {
    await attachments.update(ids => [...new Set([...ids, ...selected.map(file => file.id)])]);
    back();
  }} />;
}
