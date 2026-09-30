import { useEffect } from "react";
import { Tabs } from "ink";
import { archive, home } from "ink/icons";

declare const __inkPost: (source: string) => void;
export default function Layout() {
  useEffect(() => {
    __inkPost(JSON.stringify({ type: "ready" }));
  }, []);
  return (
    <Tabs>
      <Tabs.Screen name="index" icon={home} />
      <Tabs.Screen name="archive" icon={archive} />
    </Tabs>
  );
}
