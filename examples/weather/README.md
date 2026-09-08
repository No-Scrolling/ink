# Weather

Weather for Ink and Light Phone III, adapted from the companion Expo weather app in `~/Developer/weather`.

The example uses Open-Meteo for forecast, geocoding and air-quality data. It keeps the source app's current weather, hourly and weekly views, saved locations, units, time format, detail selection and ordering, and inverted appearance. The weather artwork is rasterised from the source app's SVGs for Ink's local `Image` API and includes light and dark variants for the appearance switch. The original artwork is from [erikflowers/weather-icons](https://github.com/erikflowers/weather-icons).

Run it from the example directory:

```sh
cd examples/weather
ink check
ink dev
```

The Locations tab lists saved places. Its Add action opens a dedicated search input page, as required by DESIGN.md. Forecast details can all be selected and appear on separate lines. The example has its own Android package ID, so it can coexist with the original app.

## Code layout

- `app/` contains pages. The root `_layout.tsx` applies saved appearance; `(tabs)/_layout.tsx` declares the tabs.
- `components/` contains shared forecast UI, weather artwork, preference loading and unit selection.
- `hooks/` handles location requests, forecast refreshes and settings saves.
- `lib/weather.ts` handles the weather APIs; `lib/preferences.ts` stores settings.
- `lib/place.ts` and `lib/routeParams.ts` handle location data and navigation parameters.

Ink generates the routes and entry point inside `.ink/`.
