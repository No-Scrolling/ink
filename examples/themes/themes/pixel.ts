import { defineTheme, defineToggle } from "ink/theme";

// Amber bitmap type with a stepped, pixel-drawn switch.
export default defineTheme({
  fonts: { body: "../assets/fonts/DepartureMono-Regular.otf" },
  colours: { background: "#120c02", foreground: "#ffb000", muted: "#7a5200" },
  text: { size: 48 },
  spacing: { inset: 72, gap: 64 },
  header: { size: 52 },
  section: { size: 32 },
  field: "inline",
  selection: "check",
  toggle: defineToggle({
    width: 120,
    height: 60,
    render: ({ on, disabled, colours }) => {
      const knob = on ? 72 : 12;
      return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 60" shape-rendering="crispEdges"><g opacity="${disabled ? 0.4 : 1}">
        <rect x="0" y="0" width="120" height="60" fill="${colours.muted}"/>
        <rect x="6" y="6" width="108" height="48" fill="${colours.background}"/>
        <rect x="${knob}" y="12" width="36" height="36" fill="${on ? colours.foreground : colours.muted}"/>
      </g></svg>`;
    },
  }),
});
