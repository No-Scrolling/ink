import photo from "../../assets/images/wills.jpeg";
import { Image, Screen } from "ink";

export default function Wills() {
  return (
    <Screen title="Wills" centered>
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
