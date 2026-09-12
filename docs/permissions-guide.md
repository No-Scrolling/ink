---
title: "Request a permission"
description: "Request access when a feature opens."
---

Request permission when the user opens the feature that needs it. Do not add a separate permission button.

## Request access

With `@ink/audio` installed, call this from the feature’s asynchronous setup:

```ts
import { microphone } from "@ink/audio/microphone";

const permission = await microphone.requestPermission();
```

| Result | What to do |
| --- | --- |
| `granted` | Start the feature. |
| `denied` | Explain why access is needed and offer a retry. |
| `blocked` | Ask the user to allow access in the phone’s app settings. |

Use [ErrorState](/screen-states#error) for a failure that prevents the screen from working. A request can also reject if the native service fails; handle that as an error, not a denial.

## Check access

`getPermission()` returns the same three values without showing a prompt:

```ts
const permission = await microphone.getPermission();
```

[Camera](/camera) and [Location](/location) also provide permission methods. After permission is granted, start the sensor or recording separately.
