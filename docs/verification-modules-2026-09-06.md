# Module verification — 6 September 2026

Manual checks used the template installed with `ink dev --device emulator-5554 --once` and the local transfer fixture. The physical LP3 was not changed. No automated tests were added.

| Scenario | Observed result | Agent-tools evidence |
| --- | --- | --- |
| Bundled SQLite | A bound query returned Central Station and Park Station from the imported database. | `image-20260906-203150-4ad0ebc4` |
| HTTP download | A 4 MiB file completed; pause and resume changed transfer state and progress. | `image-20260906-203209-e879b3da`, `image-20260906-203247-cab8c358` |
| Background transfer | The download completed while the launcher was foreground, and reopening the example reattached to it. | `image-20260906-204924-0e233f93` |
| Download persistence | A paused transfer survived force-stop/relaunch at 902 KB, then resumed beyond 3 MiB. | `image-20260906-204949-c588fb78`, `image-20260906-204955-03623b7a` |
| Managed image upload | An imported image reopened after an app rebuild, rendered, was prepared as JPEG and uploaded as multipart data. The fixture received 62,997 bytes. | `image-20260906-204831-07e8fdde` |
| File export | Save produced a 62,734-byte JPEG in Android Downloads. Share opened Android's chooser with its image preview; nothing was sent. | Manual UI and file inspection |
| Connectivity subscription | Disabling emulator Wi-Fi changed the live snapshot to cellular and metered. Wi-Fi was re-enabled afterwards. | `image-20260906-205018-93041955` |
| Recording retention | Saving two recordings added two separate `.m4a` files while retaining the earlier recording. | Before/after file listing |
| Ink media gallery | Full library permission opened a three-column gallery. Two selections produced durable attachments; swiping loaded older pages beyond the initial batch. | `image-20260906-205632-54e9cd7f`, `image-20260906-205753-91710877`, `image-20260906-210121-0d40879b` |
| Map rendering and gestures | The map filled the content area, rendered a marker and responded to panning. | `image-20260906-205607-0085b0e8`, `image-20260906-205609-c166f25f` |
| Map appearance | OpenFreeMap dark and Positron styles rendered in the corresponding Ink colour modes. | `image-20260906-205946-c77854f4`, `image-20260906-210048-05eddea9` |
| Map loading cover | Frame captures showed a uniformly white content area in light mode and uniformly black in dark mode before the map appeared. Ink owns this cover independently of MapLibre's initial overlay. | `image-20260906-210812-6b9ee2b0`, `image-20260906-210831-89c844a2` |

Auth and encrypted storage checks are recorded [separately](verification-auth-secure-store-2026-09-06.md).

After these emulator checks, the user confirmed the map's loading transition on their physical LP3, including removal of the brief “Connecting…” label. This was user verification, not an additional agent capture.

Minimal, camera-only and files-only Android Kotlin builds passed. The minimal manifest excluded file sharing and permissions; the camera-only build excluded scanner, map and audio implementations. TypeScript and Rust checks are also run during integration.

These checks do not establish reboot recovery, every server's range behaviour, permission revocation, storage exhaustion or real provider workflows. The emulator has no Custom Tabs browser; a subsequent [physical LP3 check](verification-custom-tabs.md) passed browser authentication and cancellation using the local fixture.

Image evidence is local to `.agent-tools/`; inspect a record with `scripts/agent-tools result ID --full`.
