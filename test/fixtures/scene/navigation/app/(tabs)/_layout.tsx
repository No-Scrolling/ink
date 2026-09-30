import { Tabs } from "ink";
import { home, archive } from "ink/icons";

export default function Layout() {
  return <Tabs>
    <Tabs.Screen name="index" icon={home} />
    <Tabs.Screen name="archive" icon={archive} />
  </Tabs>;
}
