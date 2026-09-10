---
title: "Troubleshooting"
description: "Resolve setup, local package and LightOS emulator problems."
---

## Missing build tools

Run `ink doctor` to identify missing tools. Follow [setup](/setup) to install them. If `ink` is not found, check that the SDK's `scripts/` directory is in your shell's `PATH`.

## Local packages

New apps use `overrides` in `package.json` to find Ink packages in the local SDK. These entries do not install every package or add unused features to the APK.

If you move the SDK, update its paths in `dependencies` and `overrides`, then run `bun install`.

For an older app without overrides:

1. Create a temporary app with `ink create` using the same SDK.
2. Copy its `overrides` object into your app's `package.json`.
3. Keep your existing dependencies and run `bun install`.

## LightOS emulator

The Light SDK emulator host and your app's LightOS configuration are separate. Check [LightOS setup](/light-sdk) if a feature reports that the SDK is not enabled.

If the host's tool configuration stops responding, try selecting **Default**, then **All tools**, in Home's Settings. This is a user-reported workaround for the emulator host, not a required step for every app.

## Duplicate home pages

`app/index.tsx` and `app/(tabs)/index.tsx` both resolve to `/`. When moving your home page into a tab group, remove the original file. See [pages and route matching](/navigation#pages).
