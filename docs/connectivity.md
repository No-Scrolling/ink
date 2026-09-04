---
title: "Connectivity"
description: "Observe network availability without treating it as request success."
tag: "Design specification"
---

`@ink/connectivity` exposes an immutable snapshot of the current network state.

```tsx
import { Text, useSnapshot } from "ink";
import { connectivity } from "@ink/connectivity";

export function ConnectionNotice() {
  const connection = useSnapshot(connectivity);
  return connection.status === "offline" ? <Text>Offline · showing saved content</Text> : null;
}
```

Snapshots distinguish unknown, offline and connected, with transport and metered state when known. Observation uses native connectivity events and pauses with the visible screen. Read a fresh snapshot on return.

Being connected does not prove that a provider is reachable or authenticated. Always handle request errors. A captive portal, DNS failure or expired account can coexist with a connected network.

Use metered state to inform download preferences. The native [download manager](downloads.md) and [background scheduler](background.md) enforce their own constraints; a JavaScript preflight check can race with a network change.

A domain sync module may use reconnect events to request reconciliation, with deduplication and backoff. Screens should not each start their own retry loop.
