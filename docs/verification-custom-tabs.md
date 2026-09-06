# LP3 Custom Tabs verification

## Initial capability inspection

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

The browser exposes Custom Tabs and colour customisation. The launch checks below verify Ink’s use of it.

## Ink integration verification

Later on 6 September, installed the debug template with `ink dev --device LP3LHMA531900140 --once` on the reserved physical phone.

| Scenario | Observed result | Agent-tools image evidence |
| --- | --- | --- |
| Open website | `example.com` opened in Chromium's `CustomTabActivity`, confirmed through the activity stack. The toolbar was black with white controls in Ink's dark appearance. | `image-20260906-211347-c7a118f0` |
| Close website | The toolbar close button returned to the same Ink screen, displaying “Returned to Ink”. | `image-20260906-211349-ab3de67b` |
| Browser authorisation | The local fixture opened in a Custom Tab; approving returned through `ink-template://oauth/callback` and completed the PKCE code exchange. Session became signed-in. | `image-20260906-211620-74e73ff7` |
| Token access | Get token displayed “Access token ready”. | `image-20260906-211649-1471cd06` |
| Cancellation | After signing out, opening browser sign-in and pressing Android Back returned with “Sign-in cancelled”. | `image-20260906-211701-c0cb05d2` |
| Cleanup | The sample session was signed-out. | `image-20260906-211714-067c7aea` |

The fixture used port 8788 through `adb reverse`; forwarding and the host server were removed afterwards. No global appearance settings were changed. The webpage retains its own styling and browser security controls remain visible. Light toolbar appearance, production identity providers and cancellation/process-death races were not exercised.
