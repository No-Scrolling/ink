import { defineTheme } from "ink/theme";

// Sizes are pixels in a 1080-pixel-wide frame. Measure them from a mockup or an `ink export` frame.
// Every setting is optional; uncomment the ones that differ from Ink's defaults.
// Run `ink theme check` to build the theme, check its contrast and render every component.
export default defineTheme({
  // Font files, relative to this file. `display` is used for screen titles.
  // fonts: { body: "./assets/fonts/Body.ttf", display: "./assets/fonts/Display.ttf" },

  // Dark mode colours. Light mode uses the inverse of each, unless `light` sets it.
  // colours: {
  //   background: "#000000",
  //   foreground: "#ffffff",
  //   muted: "#898989",
  //   light: { background: "#ffffff", foreground: "#000000", muted: "#767676" },
  // },

  // Body text size. Every size in the app scales with it.
  // text: { size: 76.5 },

  // Horizontal screen padding, and the gap between screen items and section rows.
  // spacing: { inset: 94, gap: 120 },

  // header: {
  //   layout: "centred",                            // or "large": a left-aligned title with an optional subtitle
  //   size: 51,                                     // title size; about 117 for "large"
  //   top: 32,                                      // space above the title row; 36 for "large"
  //   bottom: 32,                                   // space between the title row and the content; 56 for "large"
  //   back: { position: "start", icon: "chevron" }, // or "end" and "arrow"
  //   iconSize: 71,                                 // Back and header action icons
  //   cutout: "beside",                             // or "below": start the screen under a camera cutout
  // },

  // section: { uppercase: true, size: 43, gap: 60 },
  // subtitle: { size: 41, tone: "foreground" },     // header and toggle subtitles; tone can be "muted"
  // scrollBar: { colour: "#ffffff", visibility: "always", track: true, width: 13 }, // visibility "scrolling" fades it out

  // field: "stacked",       // or "inline": label and value on one row
  // selection: "underline", // or "check": a tick at the end of the selected row
  // toggle: "line",         // or "switch", or a control from defineToggle()

  // Themes for individual screens, used with <Screen theme="reader">. Each sets only what differs.
  // variants: {
  //   reader: { text: { size: 90 }, colours: { background: "#14110d", foreground: "#efe6d6" } },
  // },
});
