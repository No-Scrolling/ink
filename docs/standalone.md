---
title: "Create an app"
description: "Create and run a Light Phone III app."
---

Ink is a framework for building Light Phone III apps with React and TypeScript. It provides components, navigation and a built-in keyboard.

## Before you start

You need a Mac or Linux computer and a Light Phone III connected by USB, or an Android emulator. You will also need a text editor and a terminal: the app where you enter the commands in this guide.

[Set up your computer and phone](/setup) to install the `ink` command. Ink currently uses a local SDK checkout: a folder containing its source and build tools. Keep it separate from your app. There is no published installer or npm release yet.

## Create an app

Run these commands in your terminal:

```sh
ink create ~/Developer/my-app --name "My App" --package com.example.myapp
cd ~/Developer/my-app
bun install
ink check
ink devices
```

The folder must not already exist. `com.example.myapp` is the app's unique Android identifier; choose your own before distributing it.

`ink devices` lists connected devices. Copy the serial from the device you want to use:

```sh
ink dev --device emulator-5554
```

Replace `emulator-5554` with your phone's serial when using a phone. Keep this command running. The first build takes longer while build dependencies are downloaded and compiled.

You should see **Home** and **Welcome to Ink!** on your device. If the app does not open, check the terminal for errors and run `ink doctor` to check the build tools.

**Next:** [edit your first screen](/first-screen).

## Add a device feature

From your app folder, add the package you need. For example:

```sh
bun add @ink/store@0.1.0
```

The generated `package.json` points Ink packages to your local SDK. Keep its `overrides` entries when adding packages. If installation fails or you move the SDK, see [local package troubleshooting](/troubleshooting#local-packages).

See the **Modules** section for available features and [runtime compatibility](/runtime-compatibility) before adding other JavaScript libraries.
