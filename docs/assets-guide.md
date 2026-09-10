---
title: "Images and icons"
description: "Import assets and Material Symbols into your app."
---

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

Names use camel case: `more_horiz` becomes `moreHoriz`. The plain export is outlined; the `Filled` suffix selects the filled variant. Your editor can suggest exports and flag unknown names.

Use named imports. Release builds remove unused exports and generate assets only for icon references remaining in the bundle. Importing only `favoriteFilled` includes only that variant. Switching between `favorite` and `favoriteFilled` retains both. References work through props, arrays and objects as ordinary TypeScript values.

For dynamic lookups, build a small object from the icons your app supports and use `findIcon(collection, externalName)` from `ink`. It returns `undefined` for an unknown key. Avoid a namespace import used dynamically: it can retain the entire catalogue.

Ink generates icon images at 56 logical units for its standard controls. The `Icon` component's `size` controls display size; larger sizes can soften edges. Icons use weight 400; the native keyboard uses its separate weight-300 assets. Back and input-clear icons are always included.

Names starting with a digit have an `icon` prefix, such as `icon360`. JavaScript reserved names and catalogue names already ending in `Filled` have an `Icon` suffix, such as `deleteIcon`, to keep exports valid and unambiguous.

Find icon names in the [Material Symbols catalogue](https://fonts.google.com/icons). Convert the catalogue's name to the export spelling above, then use your editor’s completion to check it. A value imported from `ink/icons` is an icon reference; an arbitrary string is not a substitute.
