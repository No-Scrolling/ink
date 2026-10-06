# SDK host contracts

Run `bun test test/sdk` from the repository root. These tests exercise the actual SDK implementations in Bun. They do not install a fake QuickJS engine, replace platform globals, mock native calls, or claim platform coverage.

| Suite | Observable contract and realistic failure caught | Coverage limits |
| --- | --- | --- |
| `network-data.test.ts` | Header normalisation, duplicate cookies and injection rejection; Request ownership, clone isolation and constructor validation; Response cloning, status validation, JSON and immutable redirects; 16 MiB buffering cancellation; Blob view copying, byte slicing and managed-file offsets; FormData ordering; multipart framing and decoding against independently specified wire bytes. | Only SDK data classes and streams in the Bun host. No `fetch()` requests, redirects over the network, native stream reads, QuickJS compatibility, sockets or Android file transfers. Managed Blob tests inspect ranges without reading files. |
| `file-decoding.test.ts` | Native file metadata decodes unchanged, cancelled/removed results remain null, malformed scalar fields and unsafe byte counts fail with protocol errors, optional media metadata rejects negative/non-finite/non-numeric values. | Calls the production decoder directly; no file picking, storage persistence or media permissions. |
| `route-path.test.ts` | Whole-segment matching, exactly one percent-decoding pass, malformed escape rejection and specificity at each path depth; accepted parameter names remain captured data properties. | Pure matching helper; navigation state, layout mounting and native links require runtime integration. |

## Regression coverage

The suite protects these contracts with independently specified results:

- Multipart messages accept spaces and tabs after opening and intermediate delimiters, including `--fixture \t\r\n`, and reject other whitespace there. This follows [RFC 2046 section 5.1.1](https://www.rfc-editor.org/rfc/rfc2046.html#section-5.1.1); the wire fixtures do not use the encoder under test.
- `matchPath("/[__proto__]/[constructor]", "/note/section")` returns both accepted parameter names as own string-valued properties, preserves the ordinary object prototype and includes both fields in JSON serialisation.

Store migration/CAS/subscription behaviour, SQLite, SDK/native result envelopes, renderer integration are not covered here. Adding simulated persistence would overstate coverage; those contracts need the actual native runtime or a separately labelled protocol boundary suite.
