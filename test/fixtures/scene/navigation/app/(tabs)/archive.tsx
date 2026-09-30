import { useState } from "react";
import { Screen, Text, navigate } from "ink";

export default function Archive() {
  const [count, setCount] = useState(0);
  return <Screen title="Archive">
    <Text>Archive count {count}</Text>
    <Text onPress={() => setCount(value => value + 1)}>Increase archive</Text>
    <Text onPress={() => navigate("/detail/archive")}>Open archive detail</Text>
  </Screen>;
}
