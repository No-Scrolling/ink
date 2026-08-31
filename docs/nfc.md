# NFC

`@ink/nfc` reads one NFC tag while its screen is active. NFC does not use an Android runtime permission prompt.

## Read a tag

```tsx
import { nfcTag } from "@ink/nfc";
import { Button, Text, match } from "ink";

const tag = nfcTag({ timeoutMs: 30_000 });

{match(tag, {
  loading: () => <Text>Hold a tag near the phone</Text>,
  ready: (result) => <Text>{result.value.serialNumber}</Text>,
  error: (result) => <Text>{result.error.message}</Text>,
})}
<Button onPress={() => tag.reload()}>Read another tag</Button>
```

Reading begins when the screen becomes active and stops after one tag. It also stops when the resource reloads, the screen leaves or the app enters the background.

An interrupted read starts again when the app resumes. A ready or error result remains settled until `reload()` is called. `timeoutMs` defaults to 30 seconds and accepts 1,000 to 120,000 milliseconds.

## Tag data

The ready value contains:

- an uppercase hexadecimal `serialNumber`;
- `hasText` and `text` for the first text record;
- `hasUri` and `uri` for the first URI record;
- every NDEF record in `records`.

Each record has `{ kind, value, languageTag, mimeType, payloadBase64 }`.

| Record kind | Populated fields |
| --- | --- |
| `text` | `value`, `languageTag` |
| `uri` | `value` |
| `binary` | `payloadBase64`, and `mimeType` for MIME records |

Fields that do not apply are empty strings. Binary payloads use padded Base64 without line breaks. A tag without NDEF data still succeeds with an empty `records` list. NDEF messages are limited to 64 KiB.

## Errors and packaging

Errors distinguish unavailable hardware, disabled NFC, timeout, invalid native data, and unexpected failures.

Importing `@ink/nfc` adds the NFC permission, an optional hardware declaration and the native reader. Apps without the module carry none of them.
