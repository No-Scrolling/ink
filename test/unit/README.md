# Existing Rust test review

These eight tests were moved from inline modules without changing production behaviour. The crate modules load them with `#[cfg(test)]` and `#[path]`, preserving private access without adding public test APIs.

| Cases | Contract and realistic failure |
| --- | --- |
| Binary transport (2) | Typed-array subranges retain their bytes after mutation, payloads are consumed once, replies survive transfer, and rejected requests cannot replace accepted bytes. Catches borrowing the original buffer, accepting duplicates and retaining rejected payloads. |
| Native lists (4) | Sparse edits change the correct content/heights, scrolling bounds retained rows without JS window callbacks, keyed mutations preserve anchors/actions, and boundary requests deduplicate and release empty rows. Catches full-list mounting, stale content/heights, index-based routing and leaked row ownership. |
| Playing controls (2) | A native clock drives displayed time, speed, pause and seek without React commits; playback durations retain their units and reject invalid values. Catches a stale clock, ignored speed and treating long recordings as bounded layout dimensions. |

Binary checks now assert exact request envelopes and use an absolute wait deadline. Sparse list checks now verify rendered replacement text and every untouched row version; follow-end checks assert the event name. Expected byte sequences, labels, durations and event envelopes are literal examples, not copies of implementation algorithms.

Key-storage identity and bounded ownership checks intentionally protect allocation/windowing behaviour. The playing-clock case advances its retained start timestamp; it exercises the real engine but is not a playback-device or pixel test.
