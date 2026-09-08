import photo from "../../../assets/images/wallsocket.jpg";
import { Image, Screen } from "ink";

export default function Wallsocket() {
  return (
    <Screen title="Wallsocket" centered>
      <Image
        src={photo}
        width={349}
        height={349}
        fit="contain"
        bleed
        zoomable
      />
    </Screen>
  );
}
