---
title: "Bluetooth"
description: "Build typed Bluetooth Low Energy modules on a bounded GATT interface."
tag: "Planned"
---

`@ink/bluetooth/low-level` is the bounded escape hatch for module authors implementing Bluetooth Low Energy device protocols. Apps should usually install a domain module such as a scale, heart-rate monitor, or lock instead of handling UUIDs and packets directly.

## Build a device module

A domain module owns discovery filters, packet decoding, reconnect policy, and user-facing errors:

```tsx
const scale = SmartScale.session({
  reconnect: "while-active",
});

<Button onPress={() => scale.findAndConnect()}>Connect scale</Button>
{scale.phase === "ready" ? <Text>{scale.weightKg} kg</Text> : null}
```

The app does not need to know GATT UUIDs, byte order, MTU, operation sequencing, or notification framing. Use the low-level interface only while developing that domain module.

## Discover and connect

Create a BLE session with declared service filters and ownership:

```tsx
const connection = bleSession({
  services: [uuid16("180f")],
  owner: "screen",
  reconnect: "never",
});
```

The session owns permission, scanning, connection, service discovery, and disconnect. Creating it never opens a permission prompt. A domain view or app action calls `connection.permission.request()` explicitly.

Use `owner: "application"` only in a module whose interface promises a connection that survives navigation. Rust enforces ownership and disposes the Android adapter when the owner ends.

Device identifiers are opaque and are not stable identities unless the device protocol provides its own identifier.

## Read and write bytes

Ink provides an opaque immutable `Bytes` value with approved pure operations for slicing, concatenation, integer encoding and decoding, UTF-8, hexadecimal, and Base64 conversion.

```tsx
connection.write(modeCharacteristic, {
  data: bytes([1]),
  response: "required",
});
```

GATT reads, writes, descriptor changes, and subscription setup are queued and executed serially. Callers do not receive `busy` merely because another operation is active. Queue limits and operation timeouts are declared when the session is created.

## Receive notifications

Each subscribed characteristic exposes its own event session. Protocol deltas use bounded delivery rather than latest-only delivery:

```tsx
const measurements = connection.notifications(measurement, {
  capacity: 32,
  overflow: "error",
});
```

Delivery can be `"latest"`, `"drop-oldest"`, or `"error"`. Use `"latest"` only when values are complete replaceable snapshots. Each event contains bytes, a sequence number, monotonic receive time, and dropped count where applicable.

## Lifecycle, errors, and accessibility

Screen-owned sessions stop scanning, cancel queued GATT work, unsubscribe, and disconnect when their screen leaves. Application-owned sessions follow the domain module's documented foreground and reconnect policy.

Errors distinguish permission, unavailable or disabled Bluetooth, scan, connection, discovery, timeout, queue overflow, GATT operations, invalid packets, and unexpected failures. Low-level errors preserve operation and characteristic identity for the domain module to translate.

Domain modules should present stable connection text and user-relevant measurements. Do not announce every advertisement or packet, and do not display hardware addresses as device names.
