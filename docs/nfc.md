---
title: "NFC"
description: "Read and write supported tags through native sessions."
tag: "Design specification"
---

`@ink/nfc` provides foreground tag sessions for shortcuts, identifiers and supported NDEF content.

```ts
import { nfc } from "@ink/nfc";

const tag = await nfc.read({ signal, timeout: 30_000 });
const shortcut = decodeShortcut(tag.records);
await openShortcut(shortcut);
```

This fragment assumes a cancellation signal and app-specific validation/navigation functions. Reading waits for a tag while the app is foregrounded. Cancellation, timeout, disabled NFC and unsupported tags are distinct outcomes.

Inspect record types and validate payloads before acting. A tag URL does not authorise opening an arbitrary destination automatically. Tag identifiers are useful lookup hints, not proof of identity.

Writing takes explicit NDEF records and returns a verified result where the tag supports verification. A tag may be read-only, too small or removed mid-write. Never report success before native completion.

Sessions release on leaving the screen or backgrounding. Raw technology access belongs in a capability-specific package with documented hardware support. Reading NDEF does not imply payment access, secure-element access or card emulation.
