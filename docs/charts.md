---
title: "Charts"
description: "Render accessible line, bar, and scatter charts from typed data."
---

`@ink/charts` validates typed data and renders line, bar, and scatter charts with axes, legends, value formatting, and accessible point navigation.

## Render a line chart

Create a chart model and pass its ready value to `Chart`:

```tsx
import { Chart, chartModel } from "@ink/charts";
import { Text, match } from "ink";

const sales = chartModel({
  data: [
    { month: "Jan", online: 42, shop: 31 },
    { month: "Feb", online: 48, shop: 29 },
    { month: "Mar", online: 55, shop: 35 },
  ],
  mark: {
    kind: "line",
    x: "month",
    series: [
      { field: "online", label: "Online", pattern: "solid" },
      { field: "shop", label: "Shop", pattern: "dashed" },
    ],
    points: "extrema",
  },
  yAxis: {
    label: "Orders",
    zero: "include",
  },
  accessibility: {
    title: "Orders by month",
    description: "Online and shop orders from January to March.",
  },
});

{match(sales, {
  ready: (result) => <Chart model={result.value} height={220} />,
  error: (result) => <Text>{result.error.message}</Text>,
})}
```

`accessibility.title` is required. `height` uses Ink logical units. The chart takes its width from the surrounding layout.

For a line mark:

- `x` names a string or finite-number field.
- `series` contains up to eight numeric fields with labels.
- `points` is `"none"`, `"all"`, or `"extrema"`.
- `pattern` is `"solid"`, `"dashed"`, or `"dotted"`.
- `tone` is `"primary"`, `"secondary"`, or `"muted"`.

A `null` series value creates a gap rather than a zero.

## Render bars

Use a bar mark for category comparisons:

```tsx
const totals = chartModel({
  data: [
    { category: "Books", current: 18, previous: 12 },
    { category: "Music", current: 9, previous: 11 },
  ],
  mark: {
    kind: "bar",
    category: "category",
    series: [
      { field: "current", label: "Current" },
      { field: "previous", label: "Previous" },
    ],
    arrangement: "grouped",
    direction: "horizontal",
  },
  xAxis: { label: "Orders", zero: "include" },
  accessibility: { title: "Orders by category" },
});
```

`arrangement` is `"grouped"` by default or `"stacked"`. `direction` is `"vertical"` by default or `"horizontal"`. Category values keep their input order and must be unique.

Bar charts include zero by default. Use `zero: "automatic"` only when a non-zero baseline does not misrepresent the comparison.

## Render a scatter chart

A scatter mark uses finite numerical X and Y fields:

```tsx
const readings = chartModel({
  data: sensorReadings.value,
  mark: {
    kind: "scatter",
    x: "temperature",
    y: "pressure",
    label: "place",
  },
  xAxis: { label: "Temperature" },
  yAxis: { label: "Pressure" },
  accessibility: { title: "Pressure by temperature" },
});
```

`label` names an optional string field used to identify each point to assistive technology.

## Format axes

An axis accepts `label`, optional `domain`, optional `format`, and `zero`.

```tsx
yAxis: {
  label: "Revenue",
  domain: { minimum: 0 },
  format: { kind: "currency", currency: "GBP" },
}
```

| Format | Options |
| --- | --- |
| `number` | `maximumFractionDigits` |
| `percent` | `maximumFractionDigits`; `1` displays as 100 per cent |
| `currency` | An ISO 4217 `currency` code such as `"GBP"` |
| `duration` | `unit` as `"milliseconds"`, `"seconds"`, or `"minutes"` |

Ticks use the app locale. When labels would overlap, the chart shows fewer ticks without changing the data.

Set `legend: "hidden"` to hide a visual legend when every series is labelled elsewhere. The default is `"automatic"`.

## Data limits and errors

One chart accepts up to eight series and 5,000 records. Dense line charts are reduced to the available visual width while preserving first, last, minimum, and maximum values. Accessible point navigation retains every validated record.

`chartModel()` returns `ready` or `error` immediately. It recomputes when referenced Ink state or resource values change. Invalid data does not leave an older chart visible.

Errors distinguish empty data, missing fields, invalid values or domains, too many series or points, unsupported scales, layout, and unexpected failures. Every error has `kind`, `message`, and `retryable`, and may identify the field, series, or record that failed.

The chart is active only while visible and stops work when its screen leaves. It performs no network, file, sensor, permission, or background work.

## Accessibility

The accessibility tree exposes the chart title and description, mark type, axis labels, series labels, and every formatted value. Line summaries include first, last, minimum, and maximum values. Assistive technology can move between series and then through their points.

Colour is never the only series distinction. Charts combine tone with line patterns, point shapes, direct labels, or bar textures, and follow light, dark, and increased-contrast settings.

Use `accessibility.description` to state the conclusion that matters, such as a sustained rise or a sharp outlier. Point values alone cannot explain the business meaning of a chart.
