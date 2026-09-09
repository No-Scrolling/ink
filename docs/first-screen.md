---
title: "Edit your first screen"
description: "Make a button change the text on your phone."
---

Open your app folder in a text editor. Keep `ink dev --device <serial>` running in the terminal.

Replace `app/index.tsx` with:

```tsx
import { useState } from "react";
import { Button, Screen, Text } from "ink";

export default function Home() {
  const [count, setCount] = useState(0);
  return (
    <Screen title="My first app">
      <Text>Button presses: {count}</Text>
      <Button onPress={() => setCount(value => value + 1)}>Add one</Button>
    </Screen>
  );
}
```

Save the file. You should see **My first app**, a count and an **Add one** button. Tap the button: the number increases.

`Screen`, `Text` and `Button` are components: pieces you combine to make an interface. Their tags are JSX, the part of React that describes your screen. `onPress` tells the button what to do when tapped.

`count` is state, a value React remembers. `setCount` changes it and updates the screen. The braces in `{count}` insert its value into the text.

This count is temporary. Compatible development edits can preserve it, but a full restart starts it at zero. Later, [saved data](/saving-data) will let you keep it.

Try changing the button label. You should see the new label after saving, without manually rebuilding the app.

**Next:** [understand your project files](/project-structure).
