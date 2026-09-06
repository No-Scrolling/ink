---
title: "Connectivity"
description: "Read and watch the device’s network connection."
---

`@ink/connectivity` exposes an immutable snapshot of the current network state.

```tsx
import { Text, useSnapshot } from "ink";
import { connectivity } from "@ink/connectivity";

export function ConnectionNotice() {
  const connection = useSnapshot(connectivity);
  return connection.status === "ready" && connection.data.status === "offline"
    ? <Text>Offline · showing saved content</Text>
    : null;
}
```

The snapshot is `loading`, `ready` or `error`. Ready data has a `status` of `unknown`, `offline` or `connected`, plus the transport and metered state when available. Ink watches native events while the screen is visible and refreshes when it returns.

Use `await connectivity.get()` for a fresh one-off reading in workers or commands.

Being connected does not prove that a provider is reachable or authenticated. Always handle request errors. A captive portal, DNS failure or expired account can coexist with a connected network.

Use metered state to inform download preferences. The native [download manager](/downloads) and [background scheduler](/background) enforce their own constraints; a JavaScript preflight check can race with a network change.

Keep reconnect and retry handling in one shared module so each screen does not start a separate retry loop.
