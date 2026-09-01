import { Image, Screen } from "ink";

export default function Wallsocket() {
  return (
    <Screen title="Wallsocket" centered>
      <Image
        src="./assets/images/wallsocket.jpg"
        width={349}
        height={349}
        fit="contain"
        bleed
        zoomable
      />
    </Screen>
  );
}
