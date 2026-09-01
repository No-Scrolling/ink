---
title: "Connectivity"
description: "Observe the device's current network connection."
tag: "Planned"
---

`@ink/connectivity` reports whether Android currently has a usable network and how that connection is metered. Use it to adapt an interface, not to predict whether a request will succeed.

## Observe connectivity

Call `connectivity()` once and render its latest value:

```tsx
import { connectivity } from "@ink/connectivity";
import { Text, match } from "ink";

const connection = connectivity();

{match(connection, {
  loading: () => <Text>Checking connection</Text>,
  ready: ({ value }) => value.status === "internet" ? (
    <Text>{value.metered ? "Connected · metered" : "Connected"}</Text>
  ) : (
    <Text>Offline</Text>
  ),
  error: ({ error }) => <Text>{error.message}</Text>,
})}
```

The ready value contains:

| Field | Values | Meaning |
| --- | --- | --- |
| `status` | `"offline"`, `"local"`, `"internet"` | Whether Android has no network, a local network, or a validated internet connection. |
| `transport` | `"wifi"`, `"cellular"`, `"ethernet"`, `"other"`, `null` | The primary transport Android selected. |
| `metered` | `boolean` | Whether Android recommends limiting large transfers. |
| `roaming` | `boolean` | Whether the active mobile connection is roaming. |

An `internet` value means Android validated the active network. It does not guarantee that a particular host is reachable or that its next request will succeed.

## Adapt network-heavy features

Connectivity is useful for explaining why data is stale, suppressing automatic media loads on metered networks, or offering a retry when the device reconnects.

Do not check connectivity immediately before every request. That creates a race between the check and the request. Start the request and handle its structured Network error instead.

Use [Downloads](downloads.md) constraints for durable transfers. The Downloads implementation observes connectivity without requiring the app to coordinate it.

## Lifecycle and errors

The resource is application-scoped and shared by every declaration. It subscribes while the app is running and publishes only the latest state. Returning to the foreground refreshes the value immediately.

The module requests no runtime permission and performs no network probe of its own. Errors distinguish unavailable system connectivity information and unexpected failures. Every error provides `kind`, `message`, and `retryable`.
