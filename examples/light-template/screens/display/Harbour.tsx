import { Image, Screen } from "ink";

export default function Harbour() {
  return (
    <Screen title="Harbour" centered>
      <Image
        src="./assets/images/harbour.jpeg"
        width={349}
        height={349}
        fit="contain"
        bleed
        zoomable
      />
    </Screen>
  );
}
