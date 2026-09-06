# Account and secure storage verification — 6 September 2026

Verified the debug Ink Template on the reserved `emulator-5554`, using the local OAuth fixture on port 8788. No physical LP3 operations or new automated tests were performed.

| Scenario | Observed result | Agent-tools image evidence |
| --- | --- | --- |
| Initial account | Session showed signed-out. | `image-20260906-203905-2cd04d8a` |
| Device authorisation | The app displayed a code and verification URL, polled automatically and changed to signed-in after approval through the fixture. | `image-20260906-203928-cb56bcc6`, `image-20260906-203954-1cfcfbc8` |
| Access token | Get token succeeded after the fixture token entered its refresh window. No token appeared in the screen. | `image-20260906-204013-6b360df4` |
| Account persistence | Force-stopping and relaunching the app restored the signed-in session. | `image-20260906-204029-b808d939` |
| Local sign-out | Sign out changed the session to signed-out. | `image-20260906-204039-6f95f178` |
| Browser unavailable | Browser sign-in reported Custom Tabs unavailable and retained the Ink screen. Package-manager inspection found no Custom Tabs service; the only HTTPS handler was `org.chromium.webview_shell`. | `image-20260906-204048-512efa96` |
| Secure value write/read | Saving a sample secret made it available to a subsequent read. | `image-20260906-204149-2a2a48a4` |
| Secure value persistence | Force-stopping and relaunching the app preserved the saved secret. | `image-20260906-204210-aa58a272` |
| Secure value deletion | Removing the sample secret made a subsequent read report no saved secret. | `image-20260906-204225-dcd28f56` |

Evidence is retained in the local `.agent-tools/` directories named above. Inspect a record with `scripts/agent-tools result ID --full`. The emulator was left signed out with the sample secret removed.

The fixture also passed host-side smoke checks for device authorisation, refresh-token rotation, ranged download headers and upload byte counts. Package and template TypeScript checks passed.

A subsequent [physical LP3 Custom Tabs check](verification-custom-tabs.md) passed browser sign-in with PKCE, token access, dismissal through Android Back and dark toolbar appearance using the local fixture. The emulator's missing browser was not replaced by a WebView or full-browser fallback. These checks do not establish cross-runtime refresh races, key invalidation, cancellation races, process death during exchange or production-provider compatibility.
