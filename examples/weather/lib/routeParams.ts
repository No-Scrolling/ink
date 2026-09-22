import * as v from "valibot";

export const routeQuery = v.parser(v.fallback(v.string(), ""));
const coordinate = v.pipe(
  v.union([v.number(), v.pipe(v.string(), v.trim(), v.minLength(1))]),
  v.transform(Number),
);
const routeLocation = v.object({
  id: v.pipe(coordinate, v.safeInteger(), v.minValue(0)),
  latitude: v.pipe(coordinate, v.finite(), v.minValue(-90), v.maxValue(90)),
  longitude: v.pipe(coordinate, v.finite(), v.minValue(-180), v.maxValue(180)),
  name: v.pipe(v.fallback(v.string(), ""), v.transform(value => value || "Selected Location")),
  country: v.fallback(v.string(), ""),
  admin1: v.pipe(v.fallback(v.string(), ""), v.transform(value => value || undefined)),
});

export const decodeWeatherRoute = v.parser(v.pipe(
  v.fallback(v.nullable(routeLocation), null),
  v.transform(location => ({ location })),
));
