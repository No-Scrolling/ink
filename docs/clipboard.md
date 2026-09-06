---
title: "Clipboard"
description: "Copy and paste from explicit user actions."
---

`@ink/clipboard` reads and writes plain text through Android's clipboard.

```tsx
import { Button, Text, useAction } from "ink";
import { clipboard } from "@ink/clipboard";

export function CopyReference({ value }: { value: string }) {
  const copy = useAction(() => clipboard.writeText(value));
  return <>
    <Button onPress={() => copy.run()}>Copy reference</Button>
    {copy.status === "success" && <Text>Copied</Text>}
    {copy.status === "error" && <Text>Could not copy reference</Text>}
  </>;
}
```

`readText()` returns text or `null` when no readable text is available. Read in response to a paste action; polling the clipboard is unnecessary and can trigger system privacy behaviour.

Clipboard content can come from another app. Validate it before treating it as a URL, code or structured record. Avoid copying credentials as part of ordinary account flows. Copying a pass value does not transfer the app's metadata or issuer validity.
