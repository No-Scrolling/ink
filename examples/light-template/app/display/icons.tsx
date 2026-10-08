import { Icon, Screen, Stack } from "ink";
import { home, homeFilled } from "ink/icons";
import spotify from "../../assets/icons/spotify.svg";

export default function Icons() {
  return (
    <Screen title="Icons">
      <Stack axis="horizontal" gap={24}>
        <Icon name={home} />
        <Icon name={homeFilled} />
        <Icon name={homeFilled} tone="muted" />
      </Stack>
      <Stack axis="horizontal" gap={24}>
        <Icon name={spotify} size={24} />
        <Icon name={spotify} size={36} />
        <Icon name={spotify} size={48} />
      </Stack>
    </Screen>
  );
}
