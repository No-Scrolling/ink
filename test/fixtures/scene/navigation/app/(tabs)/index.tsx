import { useState } from "react";
import { Screen, Text, navigate } from "ink";

export default function Home() {
  const [count, setCount] = useState(0);
  return <Screen title="Home">
    <Text>Home count {count}</Text>
    <Text onPress={() => setCount(value => value + 1)}>Increase home</Text>
    <Text onPress={() => navigate("/detail/a%2Fb")}>Open encoded detail</Text>
  </Screen>;
}
