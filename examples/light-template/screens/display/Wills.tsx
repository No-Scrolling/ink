import { Image, Screen } from "ink";

export default function Wills() {
  return (
    <Screen title="Wills" centered>
      <Image
        src="./assets/images/wills.jpeg"
        width={349}
        height={349}
        fit="contain"
        bleed
        zoomable
      />
    </Screen>
  );
}
