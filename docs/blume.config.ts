import { defineConfig } from "blume";

export default defineConfig({
  title: "Ink",
  description: "Build small, fast Light Phone III apps with TypeScript.",
  feedback: false,
  logo: {
    image: {
      light: "/images/title-light.png",
      dark: "/images/title-dark.png",
      alt: "Ink",
    },
    text: "",
    href: "/get-started",
  },
  content: {
    root: ".",
    exclude: ["AGENTS.md", "README.md", "node_modules/**", "dist/**", "public/**"],
  },
  theme: {
    background: { light: "#ffffff", dark: "#0E0E10" },
    accent: {
      light: "#000000",
      dark: "#ffffff",
    },
    mode: "system",
    fonts: {
      display: {
        name: "Public Sans",
      },
      body: {
        name: "Public Sans",
      },
    },
  },
  navigation: {
    sidebar: [
      "/get-started",
      "/project-structure",
      {
        label: "Guides",
        items: ["/build-app", "/design-export", "/permissions-guide"],
      },
      {
        label: "Ink Components",
        items: [
          "/screens",
          "/text",
          "/buttons",
          "/text-input",
          "/fields",
          "/toggles",
          "/selection",
          "/images",
          "/icons",
          "/rows",
          "/lists",
          "/reordering",
          "/screen-states",
          "/confirmation",
          "/playing-screen",
          "/video",
          "/conversations",
          "/media-picker",
          "/codes",
          "/maps",
          "/canvas",
        ],
      },
      {
        label: "Reference",
        items: ["/navigation", "/data"],
      },
      {
        label: "Modules",
        items: [
          "/audio",
          "/auth",
          "/background",
          "/barcode",
          "/camera",
          "/clipboard",
          "/crypto",
          "/files",
          "/light-sdk",
          "/location",
          {
            label: "Network",
            items: ["/network", "/connectivity", "/downloads"],
          },
          "/nfc",
          "/notifications",
          "/secure-store",
          "/store",
        ],
      },
      {
        label: "Contributing and internals",
        items: ["/architecture"],
      },
    ],
    actions: [
      {
        label: "GitHub",
        href: "https://github.com/No-Scrolling/ink",
      },
    ],
  },
  deployment: {
    site: "https://ink.noscroll.ing",
  },
  redirects: [
    {
      from: "/",
      to: "/get-started",
      status: 302,
    },
    {
      from: "/install.sh",
      to: "https://raw.githubusercontent.com/No-Scrolling/ink/main/scripts/install.sh",
      status: 302,
    },
    {
      from: "/components",
      to: "/screens",
      status: 308,
    },
    {
      from: "/inputs",
      to: "/text-input",
      status: 308,
    },
  ],
});
