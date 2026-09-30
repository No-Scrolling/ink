import "@ink/network";

const hours = ["10:00", "11:00", "12:00", "13:00", "14:00", "15:00"].map(time => `2026-09-30T${time}`);
const hourly = {
  time: hours,
  temperature_2m: [17, 18, 19, 20, 19, 18],
  apparent_temperature: [16, 17, 18, 19, 18, 17],
  precipitation_probability: [10, 10, 5, 0, 5, 10],
  weather_code: [2, 2, 1, 0, 1, 2],
  is_day: [1, 1, 1, 1, 1, 1],
  ...Object.fromEntries([
    "precipitation", "wind_speed_10m", "wind_gusts_10m", "uv_index", "relative_humidity_2m",
    "dew_point_2m", "cloud_cover", "visibility", "surface_pressure",
  ].map(key => [key, hours.map(() => 0)])),
};
const daily = {
  time: ["2026-09-30", "2026-10-01", "2026-10-02"],
  temperature_2m_max: [20, 21, 18], temperature_2m_min: [12, 13, 11],
  apparent_temperature_max: [19, 20, 17], apparent_temperature_min: [11, 12, 10],
  weather_code: [2, 1, 3],
  sunrise: ["2026-09-30T07:00", "2026-10-01T07:02", "2026-10-02T07:04"],
  sunset: ["2026-09-30T18:40", "2026-10-01T18:38", "2026-10-02T18:36"],
  ...Object.fromEntries([
    "precipitation_probability_max", "uv_index_max", "precipitation_sum", "wind_speed_10m_max",
    "wind_gusts_10m_max", "relative_humidity_2m_mean", "dew_point_2m_mean", "cloud_cover_mean",
    "visibility_mean", "surface_pressure_mean",
  ].map(key => [key, [0, 0, 0]])),
};

export default function setup() {
  const fixtureFetch: typeof fetch = async input => {
    const url = String(input);
    if (url.startsWith("https://air-quality-api.open-meteo.com/")) {
      return new Response(JSON.stringify({ hourly: { time: hours, us_aqi: hours.map(() => 23),
        european_aqi: hours.map(() => 12), pm2_5: hours.map(() => 4), pm10: hours.map(() => 8) } }), { status: 200 });
    }
    if (!url.startsWith("https://api.open-meteo.com/")) throw new Error(`No design fixture for ${url}`);
    return new Response(JSON.stringify({
      current: { time: hours[0], weather_code: 2, temperature_2m: 17, apparent_temperature: 16, is_day: 1 },
      hourly, daily,
    }), { status: 200 });
  };
  Reflect.set(globalThis, "fetch", fixtureFetch);
}
