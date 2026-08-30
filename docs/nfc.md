# NFC

`@ink/nfc` provides one-shot, screen-scoped NFC tag reading.

```tsx
import { nfcTag } from "@ink/nfc";

const tag = nfcTag({ timeoutMs: 30_000 });
```

`nfcTag()` begins reading when its screen becomes active. It stops after finding one tag, when the resource reloads, when its screen becomes inactive or when the app leaves the foreground. An interrupted read rearms when the app resumes, while a ready or failed result remains settled until `reload()` is called. `timeoutMs` defaults to 30 seconds and accepts literal values from 1,000 to 120,000 milliseconds.

The ready value contains an uppercase hexadecimal `serialNumber`, convenience fields for the first text and URI records, and a homogeneous `records` list. Each record has the exact shape `{ kind, value, languageTag, mimeType, payloadBase64 }`:

- Text records set `value` and `languageTag`.
- URI records set `value`.
- Binary records set `payloadBase64` and set `mimeType` for MIME records.
- Fields which do not apply are empty strings.

Binary payloads use standard padded Base64 without line breaks. Tags without NDEF records still return successfully. Ink rejects NDEF messages larger than 64 KiB.

Errors distinguish unavailable hardware, disabled NFC, timeouts, invalid native data and unexpected failures. NFC has no Android runtime permission prompt. The compiler adds `android.permission.NFC`, an optional `android.hardware.nfc` feature declaration and the native reader only when `@ink/nfc` is imported.
