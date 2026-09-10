---
title: "Development"
description: "Run your app and refresh it as you edit."
---

From your app directory:

```sh
ink dev --device <serial>
```

Use `ink devices` to find your phone or emulator. To run from another directory, use `ink -C <app> dev`.

The first build installs and opens a development APK. Keep the command running while you edit. Ink sends JavaScript and asset updates through ADB without reinstalling the APK.

- Add `--once` to build and launch once.
- Add `--logs` to stream app logs and errors.

## Edit a screen

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

Save, then tap **Add one**. The count increases.

`Screen`, `Text` and `Button` are components: pieces you combine to make an interface. Their tags are JSX, the part of React that describes your screen. `onPress` tells the button what to do when tapped.

`count` is state, a value React remembers. `setCount` changes it and updates the screen. The braces in `{count}` insert its value into the text.

The count resets to zero when the app restarts. Follow [Save and load data](/saving-data) to keep it between sessions.

## Save and refresh

Ink automatically memoises eligible app components and hooks with Oxc’s React Compiler. Write ordinary React code; no compiler configuration is needed.

Compatible component edits preserve React state. Changing a component’s hooks remounts it. Changes to icon masks restart the UI runtime. Native code, capabilities, Android resources and app configuration rebuild the APK.

A runtime restart clears React state but keeps saved app data. The watcher includes imported files in linked packages outside your app directory.

## Add a device feature

Add packages from your app folder:

```sh
bun add @ink/store@0.1.0
```

Keep the generated `package.json` overrides. They point to your local SDK; see [package troubleshooting](/troubleshooting#local-packages).

Find packages under **Modules**. Check [runtime compatibility](/runtime-compatibility) before adding third-party libraries.

## Fix errors

A build error leaves the current app running. Fix the source and save to try again.

Uncaught runtime errors appear in a development dialog with a **Reload** action and in Logcat. Source maps point to your original code when a mapping is available. You can fix JavaScript and native build errors without restarting the command.

Errors caught by a React error boundary, and errors React recovers from, are logged without stopping the app. Your boundary can show a fallback and offer a retry. Use `useAction` for rejected save or send commands; render boundaries do not catch asynchronous event-handler failures.

In a release, an uncaught runtime error ends the JavaScript session, releases its attached native controllers and shows a close action. Detached playback retains its service lifetime.

## Background work

Each scheduled job keeps a copy of its worker bundle, so an app refresh does not change a running job’s code. Ink keeps the newest three development bundles. Worker bundles and detached audio assets have separate caches; uninstalling the app clears them.

Release builds use minified code and exclude the development reload command.

## Build a release

Use `ink check` to check your app and `ink info` to inspect its build configuration. See [release builds](/release) for signing and installation.

For another page, follow [Add another screen](/second-screen).
