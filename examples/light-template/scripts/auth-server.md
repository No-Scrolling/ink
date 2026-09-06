# Local account and transfer fixture

Run `node examples/light-template/scripts/auth-server.mjs` from the repository root. The server listens on port 8788; set `PORT` to override it. Stop it with Ctrl+C. All credentials are disposable and stored in memory.

Use a debug template build on the emulator or a connected phone. Run `adb -s SERIAL reverse tcp:8788 tcp:8788` for that device. Accounts defaults to `http://127.0.0.1:8788`, client `ink-template` and redirect `ink-template://oauth/callback`. Remove the forwarding afterwards with `adb -s SERIAL reverse --remove tcp:8788`.

For device sign-in, select **Device sign-in**, open `http://localhost:8788/verify` on the host, enter the displayed code and authorise it. Ink polls automatically until authorisation completes. Access tokens last 45 seconds; **Get token** exercises refresh, rotating the refresh token each time.

Browser sign-in uses the same fixture and requires a Custom Tabs browser on the device. Select **Browser sign-in**, authorise Ink and return via the registered custom redirect. Closing the browser without authorising cancels sign-in. HTTP is permitted only for the named local hosts in debug builds; release OAuth requires HTTPS.

To use another provider, edit the account configuration in `screens/modules/Accounts.tsx`, and register its redirect in `ink.toml` and with the provider. The example keeps provider settings in source so the screen stays focused on account actions.

`GET /download` streams a repeatable 4 MiB file in 64 KiB chunks every 100 ms. It supports `Range: bytes=N-`, `If-Range` and a stable ETag, making pause/resume observable. `POST /upload` consumes a request body and reports the received byte count. Use the emulator host address for these URLs too.

This server is a development fixture, with no production account system. It binds all local interfaces so emulator requests can reach it; do not expose its port publicly.

The [6 September verification record](../../../docs/verification-auth-secure-store-2026-09-06.md) records successful emulator device sign-in, token access and account persistence. That emulator had no Custom Tabs browser; the unavailable path passed. A subsequent [physical LP3 check](../../../docs/verification-custom-tabs.md) passed browser authorisation, redirect return, token access and cancellation.
