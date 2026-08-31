# LightOS APK capabilities absent from the public Light SDK

Research date: 30 August 2026. This is a read-only comparison of the connected LP3, the first-party APKs installed on it, LightOS releases 566–576 in `/Users/vandam/Downloads/lightos`, and `/Users/vandam/Developer/light-sdk`. The installed `com.lightos` APK is byte-for-byte the local 576 APK (SHA-256 `fe392e33d07540735693dd9e11b5e718f84ee93fed5f8e6b5cb428d33cca2ff3`).

The important result is that there are two different kinds of “missing API”. Some capabilities are ordinary Android framework calls that Ink can wrap in its own native adapter, as it already does for foreground location. Others are real private LightOS RPCs or intents and need platform support; copying their identifiers is not a supported integration.

## Strong findings

| Capability | Classification | APK evidence | Public SDK comparison | Practical conclusion |
|---|---|---|---|---|
| Cached current location from LightOS | **Private LightOS service** | `com.lightos` 574 and 576 implement custom method `GetCurrentLocation` in `com.lightos.sdk.LightOSSdkManager.resolveCustomSdkServerFunction`. `handleGetCurrentLocation()` reads `LightOSGeolocationModule.getCurrentBestLocation()` and returns optional `latitude`, `longitude`, `accuracyMeters`, and `timestampMs`. Installed `com.thelightphone.rideshare` 1.0.0 bundles `LightServiceMethod.GetCurrentLocation`; its obfuscated `p029e4.f0.w()` calls it and logs the returned fix. | `sdk/shared/.../LightServiceMethod.kt` ends at `OpenDialer`; `GetCurrentLocation` is absent from the method model and `allMethods`. No handler exists in the public server module. | This is the closest match to the requested example. It is a genuine newer/private SDK contract, not merely a location permission. The handler returns LightOS's cached HERE fix rather than starting a fresh client-side request. |
| Direct foreground location helper | **Ordinary Android framework use** | The Rideshare APK's bundled SDK also contains obfuscated class `p033f4.p` (log tag `LightLocation`). It reads GPS/network/passive last-known fixes and, if necessary, calls `LocationManager.requestLocationUpdates`; `p033f4.o` is its `LocationListener`. | No corresponding location class exists under public `sdk/client`. Fine/coarse location are only permitted metadata entries. | This can be implemented by Ink's native adapter without a LightOS-private API. It is separate from the private cached-location RPC above; the Rideshare view model actually uses the RPC. |
| Rideshare booking backend | **Private LightOS service** | The Rideshare APK bundles `GetRideshareRideEstimates`, `RequestRideshareRide`, `GetRideshareRide`, `GetRideshareActiveRides`, and `CancelRideshareRide`. `com.lightos.sdk.LightOSSdkManager` 574/576 resolves those IDs and proxies them through `LightWebClient`. Every method, including `GetCurrentLocation`, passes `untrustedRideshareCallerError()`, which requires `ClientCertType.LightSdkApproved`. | None of the five types or server handlers exists in the public SDK repository. Public `LightSdkServer.customServiceMethodResolver` is only a host extension point; it does not supply these LightOS handlers to a tool. | These are real first-party-only platform APIs, but too product-specific for Ink's general API. They also demonstrate that possessing a method ID is insufficient: LightOS enforces an approved-client certificate classification. |
| Place search/geocoding for Rideshare | **App-bundled third-party SDK, not LightOS API** | Rideshare bundles the HERE SDK. `V3.b` constructs `com.here.sdk.search.SearchEngine`; its view model uses the results for pickup and destination search. The manifest also includes HERE's private `com.here.services.internal.LocationService`. | Light SDK exposes neither place search nor routing. | Ink could expose a provider-neutral geocoding/search API later, but this evidence does not reveal a LightOS service. It would require Ink to ship or call its own provider and credentials. |
| Full camera capture and media insertion | **Ordinary Android framework/library use** | The main `com.lightos` APK has `com.lightos.camera.LightOSCameraModule`/`LightOSCameraPhotoReceiver` and uses a bundled camera library; `com.lightos.album.LightOSAlbumModule` writes images through `ContentResolver`/`MediaStore`. | The public SDK's camera surface is `LightQrCodeScanner`; it has no general photo-capture or image-library API. The metadata allowlist permits `CAMERA`, but that is build policy, not a camera API. | A general camera controller is technically possible in Ink's Android adapter. Saving media is also Android storage work, not access to a hidden LightOS provider. Permission and lifecycle design would be the main work. |
| Local alarms, timers and notifications | **Ordinary Android framework use inside the OS shell** | `com.lightos.alarm.LightOSAlarmModule`/`LightOSAlarmReceiver`, `com.lightos.timer.LightOSTimerModule`/`LightOSTimerReceiver`, and `com.lightos.notifications.LightOSNotificationsManager` use Android alarm and notification facilities. | The SDK supports remote UnifiedPush delivery and background work, but has no typed local-alarm, timer, or local-notification surface. | These are viable native-adapter capabilities only within Android's normal permission/background restrictions. The APK does not expose the LightOS alarm/timer database or UI to tools. |
| Hand an image to LightOS's screenshot editor | **Private, undocumented LightOS intent** | The 576 manifest exports `com.lightos.IntentReceiverActivity` for `android.intent.action.EDIT` with MIME `image/*`. The activity normalises the supplied URI, emits the internal React event `LightOSSystem/editScreenshot`, then opens `MainActivity`. | No matching SDK action exists. | This is callable through Android intent resolution with a granted content URI, but it is screenshot-specific, unversioned and unsupported. Treat it as a fragile platform integration, not a stable general image editor API. |

Primary APK references:

- `/Users/vandam/Downloads/lightos/576-release-lp3/decompiled/com.lightos-jadx/sources/com/lightos/sdk/LightOSSdkManager.java` — custom method routing, trust check, location response and rideshare proxy handlers.
- `/Users/vandam/Downloads/lightos/576-release-lp3/decompiled/com.lightos-jadx/sources/com/lightos/geolocation/LightOSGeolocationModule.java` — cached HERE location and continuous location engine.
- `/Users/vandam/Downloads/lightos/576-release-lp3/decompiled/com.lightos-jadx/sources/com/lightos/IntentReceiverActivity.java` and its decompiled `AndroidManifest.xml` — screenshot-edit intent.
- Installed `com.thelightphone.rideshare` 1.0.0: `p029e4.f0`, `p041h4.C0514u`, `p033f4.p`, `p033f4.o`, and `V3.b` in the pulled APK.
- `/Users/vandam/Developer/light-sdk/sdk/shared/src/main/kotlin/com/thelightphone/sdk/shared/LightServiceMethod.kt` and `/Users/vandam/Developer/light-sdk/sdk/client/src/main/kotlin` — complete public client/service comparison.

## What the manifests do **not** prove

Declared permissions and merged components overstate application capability:

- Rideshare declares fine/coarse location and genuinely uses location, but its `CAMERA`, `VIBRATE`, `WAKE_LOCK`, boot and foreground-service entries are merged SDK/library declarations; no Rideshare camera flow was found.
- Weather 6.0.0 declares camera and includes ML Kit/CameraX scanner components, yet its application code asks the user for a place name and geocodes it. It does not request device location or use the scanner.
- Authenticator 4.0.0 does use camera scanning, but it uses the SDK's existing Light QR scanner and permission activity. That is not a missing capability.
- Main `com.lightos` declares privileged telephony, contacts, calendar, APN, package-management and device-admin permissions because it is the system shell/default phone app. Those declarations do not make the facilities available to SDK tools, and the public builder does not allowlist most of them.

The public metadata allowlist should therefore be read only as “a tool may request this manifest permission”, not “Light SDK supplies this feature”. The clearest allowlisted-but-unwrapped examples are location, general camera capture, Android local notifications and wake-lock-backed work.

## Misleading or dead-looking surfaces

- `LightOSGeolocationModule.LOCATION_UPDATE_RECEIVED` is a literal `com.lightos.LOCATION_UPDATE_RECEIVED`, but no send or receive use was found in the 576 decompilation. It is not evidence of a broadcast location API.
- `LightOSDirectionsMapNavigationUpdateReceiver` is exported and names `com.lightos.directions.map.ON_LOCATION_UPDATED`, but `LightOSDirectionsNavigation` sends an **explicit** broadcast to that receiver class for in-process map updates. It is not a general location feed.
- `GetMollySocketUri` exists as an untyped custom method branch in `LightOSSdkManager` 570–576 and returns fixed MollySocket configuration. No matching client method or use was found in the inspected first-party tool APKs. It is a latent product-specific hook, not a verified general SDK capability.
- An exported component is not automatically an API. Several exported receivers/services are Android role implementations protected by platform permissions (`BIND_INCALL_SERVICE`, `BIND_DEVICE_ADMIN`, SMS delivery, WorkManager diagnostics) or are generated library components.

## Version and confidence caveats

`GetCurrentLocation`, the five rideshare methods and `OpenDialer` appear in the inspected LightOS decompilation from release 574 onward; they are absent from 566/570/572 except that `GetMollySocketUri` begins in 570. The public repository may later catch up, and private IDs can change without compatibility guarantees.

Jadx recovered the main LightOS class names cleanly but R8-obfuscated much of Rideshare. The method IDs, serializer field names, Android calls and call sites are still explicit, so the location and rideshare conclusions are high confidence. The absence claim is scoped to the checked-out `/Users/vandam/Developer/light-sdk` source, not every internal Light Phone branch.

## Recommended Ink priority

1. Keep Ink's direct Android location adapter: it is the portable route and does not depend on the trust-gated cached-location RPC.
2. Consider general photo capture and local notifications only when an Ink app needs them; both are normal Android adapters, not LightOS APIs waiting to be wrapped.
3. Do not model the rideshare RPCs or MollySocket hook as public Ink APIs.
4. Avoid depending on the screenshot-edit intent unless Light explicitly agrees to support and version it.
