---
title: "Bluetooth"
description: "Native BLE transport with TypeScript device protocols."
tag: "Design specification"
---

`@ink/bluetooth` provides BLE discovery and GATT connections. A device package turns services and characteristics into useful operations such as `readWeight()` or `setDisplayText()`.

```ts
import { bluetooth } from "@ink/bluetooth";

export async function readBattery(deviceId: string, signal?: AbortSignal) {
  const connection = await bluetooth.connect(deviceId, { signal });
  try {
    const value = await connection.read({
      service: "180f", characteristic: "2a19", signal,
    });
    if (value.length !== 1) throw new Error("Invalid battery response");
    return value[0];
  } finally {
    await connection.close();
  }
}
```

Permission and adapter readiness must be established through a user-facing connection flow. Discovery uses filters and a bounded scan window. Stop scanning after selection; a permanent discovery loop is expensive.

Values use `Uint8Array` and `DataView`. Native code serialises GATT operations, negotiates supported transfer sizes and reports disconnects. A successful write acknowledgement is not necessarily confirmation that the device performed the requested action; the device protocol defines that.

Notifications return an explicit unsubscribe function. Bound queues for ordered packets and report overflow. Device modules own framing, response correlation and reconnect reconciliation. They should expose meaningful snapshots and commands to screens.

A remembered device ID is a hint for reconnecting, not an active handle or a guarantee that the device is still paired. Foreground attachments release with their owner. Background connections require an explicitly supported native service; ordinary React hooks do not keep them alive.

BLE transport does not imply Bluetooth Classic, audio routing or arbitrary profiles. Those capabilities need their own integration.
