import { ScrollView, StyleSheet, Text, View } from "react-native";
import { n } from "@/utils/scaling";

const items = Array.from({ length: 1_000 }, (_, index) => `Item ${index + 1}`);

export default function Items() {
  return (
    <View style={styles.screen}>
      <Text style={styles.header}>Items</Text>
      <ScrollView contentContainerStyle={styles.content} overScrollMode="never">
        {items.map((item) => (
          <Text key={item} style={styles.item}>{item}</Text>
        ))}
      </ScrollView>
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
    gap: n(14),
    paddingHorizontal: n(37),
    paddingBottom: n(20),
  },
  item: {
    color: "white",
    fontFamily: "PublicSans-Regular",
    fontSize: n(30),
    lineHeight: n(45),
  },
});
