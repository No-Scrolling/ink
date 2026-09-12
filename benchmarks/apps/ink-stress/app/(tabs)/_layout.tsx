import { Tabs } from "ink";
import { homeFilled, settingsFilled } from "ink/icons";

export default function Layout() {
  return <Tabs>
    <Tabs.Screen name="index" icon={homeFilled} />
    <Tabs.Screen name="settings" icon={settingsFilled} />
  </Tabs>;
}
