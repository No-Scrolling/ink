# LP3 Custom Tabs capability inspection

6 September 2026. Read-only ADB inspection of the connected physical Light Phone III, serial `LP3LHMA531900140`, running Android 14. No app was installed or launched, and no device settings were changed.

## Findings

- Chromium package: `org.chromium.chrome`.
- Version: `133.0.6888.0` (version code `688800004`).
- Querying `android.support.customtabs.action.CustomTabsService` returned `org.chromium.chrome/.browser.customtabs.CustomTabsConnectionService`.
- The service advertises `androidx.browser.customtabs.category.ColorSchemeCustomization` and `androidx.browser.customtabs.category.NavBarColorCustomization`.
- Chromium declares `CustomTabActivity` and `TranslucentCustomTabActivity`.
- Resolving a browsable HTTPS intent selected Chromium's `IntentDispatcher`.

## Commands

```sh
adb -s LP3LHMA531900140 shell cmd package query-services --brief -a android.support.customtabs.action.CustomTabsService
adb -s LP3LHMA531900140 shell cmd package resolve-activity --brief -a android.intent.action.VIEW -c android.intent.category.BROWSABLE -d https://example.com
adb -s LP3LHMA531900140 shell dumpsys package org.chromium.chrome
adb -s LP3LHMA531900140 shell getprop ro.build.version.release
```

## What this establishes

The installed browser advertises the native service needed for Custom Tabs, with appearance customisation. This supports using a browser-backed in-app window for links and sign-in.

Service discovery does not establish successful binding, visible presentation, exact toolbar styling, Back behaviour or OAuth redirect handling. Those require an Ink integration and an actual launch on the phone. Do not infer that trusted-web-activity capabilities allow arbitrary OAuth pages to hide browser security UI.
