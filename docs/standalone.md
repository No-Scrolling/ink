---
title: "Make an app for your Light Phone"
description: "Start with one screen, then build something useful."
---

Ink lets you build Light Phone III apps with TypeScript and React. You combine components such as text, buttons and lists; Ink handles their appearance, navigation and keyboard interactions.

You can build a personal tool, a weather app, a music player or something entirely your own. You do not need to design every screen from scratch.

## Before you start

You need a Mac or Linux computer and a Light Phone III connected by USB, or an Android emulator. You will also need a text editor and a terminal: the app where you enter the commands in this guide.

Ink currently runs from a local SDK checkout—a folder containing the tools used to build your app. There is no published Ink installer or npm release yet. Keep this folder separate from the apps you create.

[Set up your computer and phone](/setup) first. It establishes the `ink` command used throughout these docs.

## Create an app

In your terminal, run:

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

You should see **Home** and **Welcome to Ink!** on your device. If you do not, read the terminal error and run `ink doctor` to check the toolchain.

**Next:** [edit your first screen](/first-screen).

## Add a device feature

From your app folder, add the package you need. For example:

```sh
bun add @ink/store@0.1.0
```

New projects include package overrides that resolve Ink modules from your SDK checkout, including their dependencies. Overrides do not install every module or add unused features to your APK. Use the package's documented import, such as `@ink/audio/microphone` for pitch analysis.

Keep the generated overrides when editing `package.json`. If you move the SDK, update its local paths. Projects created before these overrides were added can copy the `overrides` object from a newly created app using the same SDK. Keep your existing dependencies and app code.

See the **Modules** section for available features and [runtime compatibility](/runtime-compatibility) before adding other JavaScript libraries.
