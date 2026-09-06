import { useState } from "react";
import { StyleSheet, Text, View } from "react-native";
import { Header } from "@/components/Header";
import { StyledButton } from "@/components/StyledButton";
import { n } from "@/utils/scaling";

export default function Counter() {
  const [count, setCount] = useState(0);

  return (
    <View style={styles.screen}>
      <Header headerTitle="Counter" hideBackButton />
      <View style={styles.content}>
        <Text style={styles.count}>Count: {count}</Text>
        <StyledButton text="Increase" onPress={() => setCount((value) => value + 1)} />
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  screen: {
    flex: 1,
    backgroundColor: "black",
  },
  content: {
    flex: 1,
    alignItems: "center",
    justifyContent: "center",
    gap: n(16),
    paddingBottom: n(10),
  },
  count: {
    color: "white",
    fontFamily: "PublicSans-Regular",
    fontSize: n(40),
  },
});
