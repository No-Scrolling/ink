# Run the local account and transfer server

Use this server to try sign-in, downloads and uploads without production credentials. Accounts and tokens are temporary and stay in memory.

## Start the server

From the repository root, run:

```sh
node examples/light-template/scripts/auth-server.mjs
```

The server uses port 8788. Set `PORT` to change it, and press Ctrl+C to stop it. It listens on all local interfaces for emulator access; do not expose it publicly.

## Connect an account example

Use a debug template build and forward the device’s port to your computer:

```sh
adb -s SERIAL reverse tcp:8788 tcp:8788
```

Replace `SERIAL` with your emulator or phone serial. Accounts uses `http://127.0.0.1:8788`, client `ink-template` and redirect `ink-template://oauth/callback`.

- **Device sign-in:** open `http://localhost:8788/verify` on your computer, enter the displayed code and authorise it. Ink polls until approval.
- **Browser sign-in:** select the action and authorise Ink in the Custom Tab. The redirect returns to Ink. Closing the tab cancels sign-in. This needs a Custom Tabs browser on the device.
- **Get token:** request the current token. Tokens expire after 45 seconds; later requests exercise refresh-token rotation.

When finished, remove the forwarding:

```sh
adb -s SERIAL reverse --remove tcp:8788
```

Debug OAuth permits local HTTP endpoints. Release OAuth requires HTTPS. To use a real provider, update `app/modules/accounts.tsx` and register the same redirect in `ink.toml` and with the provider.

## Try transfers

The Files and Downloads examples use the emulator’s host address, `http://10.0.2.2:8788`.

| Endpoint | Behaviour |
| --- | --- |
| `GET /download` | Sends a 4 MiB file in 64 KiB chunks every 100 ms. Supports `Range`, `If-Range` and a stable ETag for pause and resume. |
| `POST /upload` | Reads the request body and returns the received byte count. |
