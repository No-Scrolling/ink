import { Screen } from "ink";
import { Video } from "@ink/video";
import video from "../../assets/videos/test.mov";

export default function LocalVideo() {
  return (
    <Screen title="Local video">
      <Video src={video} />
    </Screen>
  );
}
