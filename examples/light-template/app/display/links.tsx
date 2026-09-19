import { LinkPreview, Screen, Text } from "ink";

export default function Links() {
  return (
    <Screen title="Links">
      <Text>
        Read the <Text href="https://ink.noscroll.ing">docs</Text>.
      </Text>
      <LinkPreview
        url="https://www.youtube.com/watch?v=-zjJpFYtx9s"
        title="Arson as a Christmas Tradition: The Gävle Goat"
        image={{ src: "https://i.ytimg.com/vi/-zjJpFYtx9s/hqdefault.jpg", width: 480, height: 360 }}
      />
    </Screen>
  );
}
