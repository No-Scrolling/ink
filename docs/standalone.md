---
title: "Create an app"
description: "Create and run a Light Phone III app."
---

Ink is a framework for building Light Phone III apps with React and TypeScript. It provides components, navigation and a built-in keyboard.

## Before you start

Complete [setup](/setup) first. Ink currently runs from a local SDK checkout; there is no published installer or npm release yet.

## Create an app

```sh
ink create ~/Developer/my-app --name "My App" --package com.example.myapp
cd ~/Developer/my-app
bun install
ink check
ink devices
```

The folder must not already exist. `com.example.myapp` is the app's unique Android identifier; choose your own before distributing it.

Use the device serial listed by `ink devices`:

```sh
ink dev --device emulator-5554
```

Replace `emulator-5554` with your device’s serial. Keep `ink dev` running while editing.

The app opens to **Home**, displaying **Welcome to Ink!**.

**Next:** [edit your first screen](/first-screen).

## Add a device feature

Add packages from your app folder:

```sh
bun add @ink/store@0.1.0
```

Keep the generated `package.json` overrides. They point to your local SDK; see [package troubleshooting](/troubleshooting#local-packages).

Find packages under **Modules**. Check [runtime compatibility](/runtime-compatibility) before adding third-party libraries.
