---
title: "Bluetooth"
description: "Discover and communicate with Bluetooth Low Energy devices."
tag: "Planned"
---

`@ink/bluetooth` discovers nearby Bluetooth Low Energy devices, connects to one device, and reads, writes, or subscribes to its GATT characteristics through one controller.

## Request permission

Read and request nearby-device permission before scanning:

```tsx
import { bluetoothPermission } from "@ink/bluetooth";
import { Button, Text, match } from "ink";

const permission = bluetoothPermission();

{match(permission, {
  loading: () => <Text>Checking Bluetooth access</Text>,
  ready: (result) => <Text>{result.value}</Text>,
  error: (result) => <Text>{result.error.message}</Text>,
})}
<Button onPress={() => permission.request()}>Allow Bluetooth access</Button>
```

The ready value is `"granted"`, `"denied"`, `"blocked"`, or `"unknown"`. Creating the permission resource or starting a scan never opens a prompt. Call `request()` from a user action.

Permission does not enable Bluetooth. Operations return a `disabled` error while the device radio is off.

## Discover devices

Create one controller for scanning, connecting, and GATT work:

```tsx
import { bluetoothController } from "@ink/bluetooth";
import { Button, Text } from "ink";

const bluetooth = bluetoothController();

<Button onPress={() => bluetooth.startScan({
  serviceUuids: ["180f"],
  timeoutMs: 15_000,
})}>
  Find battery devices
</Button>

{bluetooth.scan.devices.map((device) => (
  <Button onPress={() => bluetooth.connect(device.id)}>
    {device.name ?? "Unnamed device"} {device.rssi} dBm
  </Button>
))}
```

`serviceUuids` filters advertised services. `namePrefix` filters the advertised name. UUIDs may use 16-bit form such as `"180f"` or lower-case canonical form such as `"0000180f-0000-1000-8000-00805f9b34fb"`.

`timeoutMs` accepts 1,000 to 120,000 and defaults to 15,000 milliseconds. Reaching the scan timeout returns the scan to `idle`; it is not an error. Call `stopScan()` to stop earlier.

The scan state contains:

| Status | Meaning |
| --- | --- |
| `idle` | No scan is active. Previous results remain available. |
| `scanning` | Results update as advertisements arrive. |
| `error` | Scanning failed. Previous results remain available. |

Each device has an opaque `id`, optional `name`, `rssi`, advertised service UUIDs, `connectable`, and `lastSeenAtMs`. Device IDs belong to the active controller and cannot be persisted as a stable identity.

## Connect to a device

Call `connect(device.id)` from a discovered device. Connecting stops the active scan and discovers the device's services and characteristics.

| Status | Meaning |
| --- | --- |
| `disconnected` | No device is connected. |
| `connecting` | A link is opening. |
| `discovering` | The link is open and GATT discovery is running. |
| `connected` | `services` contains the complete service snapshot. |
| `error` | Connection or discovery failed. |

Pass `{ timeoutMs }` as the second argument to `connect()` to change the 15-second connection timeout. Call `disconnect()` to cancel connection work, unsubscribe, clear services, and close the link.

Each service contains its UUID and characteristics. A characteristic contains its service UUID, characteristic UUID, and any supported `read`, `write`, `write-without-response`, `notify`, or `indicate` properties.

## Read and write a characteristic

Use the service and characteristic UUIDs shown after discovery:

```tsx
const batteryLevel = {
  serviceUuid: "180f",
  characteristicUuid: "2a19",
};

<Button onPress={() => bluetooth.read(batteryLevel)}>
  Read battery
</Button>

<Text>{bluetooth.operation.status === "ready"
  ? bluetooth.operation.dataBase64
  : "No battery reading"}</Text>
```

Reads and writes are serial. Starting another while one is running returns a `busy` error and leaves the active operation unchanged.

Write bytes as padded Base64 without line breaks:

```tsx
<Button onPress={() => bluetooth.write(mode, {
  dataBase64: "AQ==",
  response: "required",
})}>
  Set mode
</Button>
```

`response` is `"required"` by default or `"not-required"` for a characteristic that supports unacknowledged writes. Read and write timeouts default to 10 seconds.

The package sends one characteristic value per write. Split or combine values according to the device protocol before calling `write()`.

## Subscribe to notifications

Call `subscribe(characteristic)` after the controller is connected:

```tsx
<Button onPress={() => bluetooth.subscribe(measurement)}>
  Start measurements
</Button>

<Text>{bluetooth.notifications.sample?.dataBase64 ?? "No measurement"}</Text>
```

The notification stream keeps only the latest event. `sequence` increments for every event, including repeated values. Each sample contains the characteristic, Base64 data, and `receivedAtMs`.

Several characteristics can be subscribed on one connection. Call `unsubscribe(characteristic)` to stop one of them.

## Lifecycle and errors

The controller belongs to its declaring screen. Leaving the screen stops scanning, cancels GATT work, unsubscribes, and disconnects. Moving the app to the background stops scanning and pauses notifications before closing the connection. Return to the screen and call `connect()` again when needed.

The controller does not reconnect automatically after an unexpected link loss. Late updates from a closed or replaced connection are ignored.

Errors distinguish permission, unavailable or disabled Bluetooth, scan, connection, timeout, GATT, unsupported operations, invalid data, busy state, and unexpected failures. Every error has `kind`, `message`, `retryable`, and an operation such as `"scan"`, `"connect"`, `"read"`, or `"write"`.

## Accessibility

Show text for scanning, connecting, connected, and disconnected states. Give unnamed devices a screen-local label such as `Device 1`; do not display a hardware address.

Do not announce every advertisement or notification. Announce only stable connection changes and user-relevant measurements at a suitable rate.
