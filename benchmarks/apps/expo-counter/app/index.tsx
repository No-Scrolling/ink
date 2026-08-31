import { useState } from "react";
import { Pressable, StyleSheet, Text, View } from "react-native";
import { n } from "@/utils/scaling";

export default function Counter() {
  const [count, setCount] = useState(0);

  return (
    <View style={styles.screen}>
      <Text style={styles.header}>Counter</Text>
      <View style={styles.content}>
        <Text style={styles.count}>Count: {count}</Text>
        <Pressable onPress={() => setCount((value) => value + 1)} style={styles.button}>
          <Text style={styles.buttonText}>Increase</Text>
        </Pressable>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  screen: {
    flex: 1,
    backgroundColor: "black",
  },
  header: {
    height: n(82),
    color: "white",
    fontFamily: "PublicSans-Regular",
    fontSize: n(25),
    lineHeight: n(82),
    textAlign: "center",
  },
  content: {
    flex: 1,
    alignItems: "center",
    justifyContent: "center",
    gap: n(16),
  },
  count: {
    color: "white",
    fontFamily: "PublicSans-Regular",
    fontSize: n(40),
  },
  button: {
    backgroundColor: "white",
    paddingHorizontal: n(32),
    paddingVertical: n(18),
  },
  buttonText: {
    color: "black",
    fontFamily: "PublicSans-Regular",
    fontSize: n(30),
  },
});
