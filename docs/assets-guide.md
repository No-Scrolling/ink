---
title: "Choose components, images and icons"
description: "Build familiar screens with Ink’s defaults."
---

Start with the component that matches the interaction:

| What you need | Component |
| --- | --- |
| A page and its header | `Screen` |
| Text or a labelled action | `Text` or `Button` |
| A setting with its current value | `Field` |
| A tappable title, subtitle and optional image | `Row` |
| A switch or a choice | `Toggle` or `Select` |
| A collection that can scroll | `List` |
| A small group within a page or row | `Stack` |

Use `List` for growing collections so Ink only renders the visible window. Use a `Stack` for a few related elements, such as a temperature and its label. See [components](/components) and [lists](/lists) for examples.

## Match the layout

Let `Screen` own page padding and navigation. Use `centered` for a centred screen; do not add invisible text or empty rows as spacers. Screens and lists use the same default gap. Set `gap={0}` deliberately for compact content such as forecast rows.

Sizes are logical layout units, not screenshot pixels. A screenshot can be scaled by the device density or emulator window. Begin with Ink’s defaults and check the result on the phone or emulator.

## Import assets

Import local images and MP3 audio. Use HTTPS or managed-file URLs for runtime sources.

```tsx
import photo from './photo.jpg';
import track from './track.mp3';
<Image src={photo} width={200} height={200} />
```

## Import icons

Import Material Symbols directly from `ink/icons`:

```tsx
import { settings, favorite, favoriteFilled } from "ink/icons";
import { Icon, Button } from "ink";

<Button icon={settings}>Settings</Button>
<Icon name={active ? favoriteFilled : favorite} size={28} />
```

Names use camel case: `more_horiz` becomes `moreHoriz`. The plain export is outlined; the `Filled` suffix selects the filled variant. TypeScript provides completion and catches unknown exports without a separate collection file or generation step.

Use named imports. Release builds remove unused exports and generate assets only for icon references remaining in the bundle. Importing only `favoriteFilled` includes only that variant. Switching between `favorite` and `favoriteFilled` retains both. References work through props, arrays and objects as ordinary TypeScript values.

For dynamic lookups, build a small object from the icons your app supports and use `findIcon(collection, externalName)` from `ink`. It returns `undefined` for an unknown key. Avoid a namespace import used dynamically: it can retain the entire catalogue.

Ink owns raster resolution and generates icons at 56 logical units, covering its standard controls. The `Icon` component's `size` controls display size; larger sizes can soften edges. Icons use weight 400; the native keyboard uses its separate weight-300 assets. Back and input-clear icons are always included.

Names starting with a digit have an `icon` prefix, such as `icon360`. JavaScript reserved names and catalogue names already ending in `Filled` have an `Icon` suffix, such as `deleteIcon`, to keep exports valid and unambiguous.


Find icon names in the [Material Symbols catalogue](https://fonts.google.com/icons). Convert the catalogue's name to the export spelling above, then use your editor’s completion to check it. A value imported from `ink/icons` is an icon reference; an arbitrary string is not a substitute.
