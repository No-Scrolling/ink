import photo from "../../assets/images/harbour.jpeg";
import { Image, Screen } from "ink";

export default function Harbour() {
  return (
    <Screen title="Harbour" centered>
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
