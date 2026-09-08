import { Tabs } from "ink";
import { homeFilled, paletteFilled, settingsFilled, widgetsFilled } from "ink/icons";

export default function Layout() {
  return <Tabs>
    <Tabs.Screen name="index" icon={homeFilled} />
    <Tabs.Screen name="modules" icon={widgetsFilled} />
    <Tabs.Screen name="examples" icon={paletteFilled} />
    <Tabs.Screen name="settings" icon={settingsFilled} />
  </Tabs>;
}
