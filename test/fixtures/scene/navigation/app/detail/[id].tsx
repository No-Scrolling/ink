import { Screen, Text, navigate, replace, useRouteParams } from "ink";

export default function Detail() {
  const { id } = useRouteParams("/detail/[id]");
  return <Screen title="Detail">
    <Text>Detail id {id}</Text>
    <Text onPress={() => replace("/confirmation")}>Replace detail</Text>
    <Text onPress={() => navigate("/archive")}>Return to archive tab</Text>
  </Screen>;
}
