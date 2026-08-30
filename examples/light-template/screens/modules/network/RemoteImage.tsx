import { Image, Screen } from "ink";

export default function RemoteImage() {
  return (
    <Screen title="Remote Image" centered>
      <Image
        src="https://a5.mzstatic.com/us/r1000/0/Music211/v4/51/41/9c/51419c3f-dce3-5072-eb94-fc453f6fdb87/199350974793.jpg"
        bleed
        width={349}
        height={349}
        fit="contain"
      />
    </Screen>
  );
}
