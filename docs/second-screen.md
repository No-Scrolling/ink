---
title: "Add another screen"
description: "Connect two pages with a button."
---

Create `app/about.tsx`:

```tsx
import { Screen, Text } from "ink";

export default function About() {
  return <Screen title="About"><Text>Made for my Light Phone.</Text></Screen>;
}
```

In `app/index.tsx`, add `navigate` to your Ink import and place this button inside `Screen`:

```tsx
<Button onPress={() => navigate("/about")}>About this app</Button>
```

Save and tap **About this app**. Ink opens the new page and supplies the header's back action. The path `/about` comes from the filename `app/about.tsx`.

Keep navigation in callbacks such as `onPress`, rather than running it while a component renders.

## Pass a value to a page

A results page can receive a search query:

```tsx
navigate({ path: "/results", params: { query: "London" } });
```

In `app/results.tsx`, read it with `useRouteParams<{ query: string }>()`. Validate external or optional values before using them. For a place preview, you can pass serialisable coordinates and a label as parameters; you do not need to save the place just to open its forecast.

See [navigation reference](/navigation) for complete parameter decoding, dynamic pages, shared layouts and tabs.

**Next:** [save and load data](/saving-data).
