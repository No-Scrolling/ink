// Evaluates an app's theme module and prints a JSON description for the Rust compiler.
// Custom controls are rendered to SVG for each state here, so no theme code runs on the phone.
import { dirname, relative, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [themePath] = Bun.argv.slice(2);
const file = relative(process.cwd(), themePath);
const fail = message => {
  console.error(`${file}: ${message}`);
  process.exit(1);
};

let theme;
try {
  theme = (await import(pathToFileURL(themePath).href)).default;
} catch (error) {
  fail(`could not load the theme: ${error?.message ?? error}`);
}
if (theme == null || typeof theme !== "object") fail("export the theme as the default export: `export default defineTheme({ … })`");

// Each key maps to its allowed child keys, or null for a value. Keep this in step with the `Theme`
// type and the Rust `ThemeSource`; test/fixtures/scene/themes/theme.ts sets every key to check all three.
const inputStyle = Object.fromEntries([
  "border", "borderWidth", "borderColour", "radius", "background", "paddingX", "paddingY", "minHeight",
  "fontSize", "font", "align", "textColour", "placeholderColour", "affixColour", "cursorColour",
  "selectionColour", "clearColour", "clearSize", "clearPadding", "clearGap",
].map(key => [key, null]));
const shape = {
  fonts: { body: null, display: null, input: null },
  colours: { background: null, foreground: null, muted: null, light: { background: null, foreground: null, muted: null } },
  text: { size: null },
  spacing: { inset: null, gap: null, rowGap: null },
  header: { layout: null, size: null, top: null, bottom: null, cutout: null, back: { position: null, icon: null }, iconSize: null },
  section: { uppercase: null, size: null, gap: null },
  scrollBar: { colour: null, visibility: null, track: null, width: null, inset: null },
  subtitle: { size: null, tone: null },
  row: { size: null },
  toggle: null,
  selection: null,
  field: null,
  hitSlop: null,
  textInput: { ...inputStyle, focused: inputStyle, invalid: inputStyle, disabled: inputStyle, readOnly: inputStyle },
};
const aliases = { colors: "colours", color: "colour", centered: "centred", font: "fonts" };

function closest(key, options) {
  const distance = (a, b) => {
    const row = Array.from({ length: b.length + 1 }, (_, index) => index);
    for (let i = 1; i <= a.length; i++) {
      let previous = row[0];
      row[0] = i;
      for (let j = 1; j <= b.length; j++) {
        const current = row[j];
        row[j] = Math.min(row[j] + 1, row[j - 1] + 1, previous + (a[i - 1] === b[j - 1] ? 0 : 1));
        previous = current;
      }
    }
    return row[b.length];
  };
  const best = options.map(option => [option, distance(key, option)]).sort((a, b) => a[1] - b[1])[0];
  return best && best[1] <= 2 ? best[0] : undefined;
}

function check(value, expected, path) {
  for (const key of Object.keys(value)) {
    if (key in expected) {
      if (expected[key] && value[key] != null) {
        if (typeof value[key] !== "object") fail(`\`${path}${key}\` must be an object`);
        check(value[key], expected[key], `${path}${key}.`);
      }
      continue;
    }
    if (aliases[key] && aliases[key] in expected) {
      value[aliases[key]] = value[key];
      delete value[key];
      check(value, expected, path);
      return;
    }
    const suggestion = closest(key, Object.keys(expected));
    fail(`unknown key \`${path}${key}\`${suggestion ? `; did you mean \`${path}${suggestion}\`?` : `. Valid keys: ${Object.keys(expected).join(", ")}`}`);
  }
}
check(theme, { ...shape, variants: null }, "");
if (theme.variants !== undefined && (theme.variants === null || typeof theme.variants !== "object")) fail("`variants` must be an object of named themes");
const { variants = {}, ...base } = theme;
for (const [name, variant] of Object.entries(variants)) {
  if (!/^[a-z][a-zA-Z0-9-]*$/.test(name)) fail(`variant name ${JSON.stringify(name)} must start with a lowercase letter and use only letters, digits and hyphens`);
  if (variant === null || typeof variant !== "object") fail(`\`variants.${name}\` must be an object`);
  check(variant, shape, `variants.${name}.`);
}

// A variant overrides the app theme key by key. Controls from defineToggle() replace the whole value.
const merge = (target, source) => {
  if (source === undefined) return target;
  const plain = value => value !== null && typeof value === "object" && !Array.isArray(value) && value.kind === undefined;
  if (!plain(target) || !plain(source)) return source;
  const merged = { ...target };
  for (const [key, value] of Object.entries(source)) merged[key] = merge(target[key], value);
  return merged;
};

// Paths in messages name the variant being described.
let prefix = "";

const number = (value, path, { min = 0, max = Infinity } = {}) => {
  if (value === undefined) return undefined;
  if (typeof value !== "number" || !Number.isFinite(value) || value < min || value > max) {
    fail(`\`${prefix}${path}\` must be a number from ${min}${max === Infinity ? "" : ` to ${max}`}, got ${JSON.stringify(value)}`);
  }
  return value;
};
const choice = (value, path, options) => {
  if (value === undefined) return undefined;
  if (!options.includes(value)) fail(`\`${prefix}${path}\` must be ${options.map(option => `"${option}"`).join(" or ")}, got ${JSON.stringify(value)}`);
  return value;
};
const colourPattern = /^#([0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i;
const roles = (value = {}, path) => {
  for (const [key, colour] of Object.entries(value)) {
    if (key === "light") continue;
    if (typeof colour !== "string" || !colourPattern.test(colour)) fail(`\`${prefix}${path}${key}\` must be a hex colour such as "#8a8a8a", got ${JSON.stringify(colour)}`);
  }
  const { background, foreground, muted } = value;
  return { background, foreground, muted };
};

async function describe(theme) {
  const fonts = {};
  for (const role of ["body", "display", "input"]) {
    const path = theme.fonts?.[role];
    if (path === undefined) continue;
    if (typeof path !== "string") fail(`\`${prefix}fonts.${role}\` must be a path to a .ttf or .otf file`);
    const absolute = resolve(directory, path);
    if (!(await Bun.file(absolute).exists())) fail(`\`${prefix}fonts.${role}\`: could not find ${path}`);
    fonts[role] = absolute;
  }

  const dark = roles(theme.colours, "colours.");
  const light = theme.colours?.light ? roles(theme.colours.light, "colours.light.") : undefined;
  // Ink inverts colours in linear light, so derived light colours match the app's light mode.
  // Muted text inverts in sRGB so it keeps its contrast against the light background.
  const invert = (colour, srgb = false) => {
    const hex = colour.slice(1);
    const full = hex.length === 3 ? [...hex].map(digit => digit + digit).join("") : hex.slice(0, 6);
    const toLinear = value => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
    const toSrgb = value => value <= 0.0031308 ? value * 12.92 : 1.055 * value ** (1 / 2.4) - 0.055;
    return "#" + [0, 2, 4].map(index => {
      const value = parseInt(full.slice(index, index + 2), 16) / 255;
      const channel = srgb ? 1 - value : toSrgb(1 - toLinear(value));
      return Math.round(channel * 255).toString(16).padStart(2, "0");
    }).join("");
  };
  const resolved = scheme => {
    const base = { background: dark.background ?? "#000000", foreground: dark.foreground ?? "#ffffff", muted: dark.muted ?? "#898989" };
    if (scheme === "dark") return base;
    return Object.fromEntries(Object.entries(base).map(([key, colour]) => [key, light?.[key] ?? invert(colour, key === "muted")]));
  };

  let toggle;
  if (theme.toggle === undefined || typeof theme.toggle === "string") {
    if (theme.toggle !== undefined && !["line", "switch"].includes(theme.toggle)) fail(`\`${prefix}toggle\` must be "line", "switch" or a control from defineToggle(), got ${JSON.stringify(theme.toggle)}`);
    toggle = theme.toggle === undefined ? undefined : { style: theme.toggle };
  } else if (theme.toggle?.kind === "toggle") {
    const width = number(theme.toggle.width, "toggle.width", { min: 1, max: 1080 });
    const height = number(theme.toggle.height, "toggle.height", { min: 1, max: 1240 });
    if (typeof theme.toggle.render !== "function") fail(`\`${prefix}toggle.render\` must be a function returning SVG markup`);
    // Off, on, disabled off and disabled on, for each scheme.
    const svgs = ["dark", "light"].map(scheme => [[false, false], [true, false], [false, true], [true, true]].map(([on, disabled]) => {
      const svg = theme.toggle.render({ on, disabled, scheme, colours: resolved(scheme) });
      if (typeof svg !== "string" || !svg.trimStart().startsWith("<svg")) fail(`\`${prefix}toggle.render\` must return SVG markup starting with <svg; got ${JSON.stringify(String(svg).slice(0, 40))} for on: ${on}, disabled: ${disabled}, scheme: "${scheme}"`);
      return svg;
    }));
    toggle = { style: "custom", width, height, svgs };
  } else {
    fail(`\`${prefix}toggle\` must be "line", "switch" or a control from defineToggle()`);
  }

  if (theme.section?.uppercase !== undefined && typeof theme.section.uppercase !== "boolean") fail(`\`${prefix}section.uppercase\` must be true or false`);
  if (theme.scrollBar?.colour !== undefined && (typeof theme.scrollBar.colour !== "string" || !colourPattern.test(theme.scrollBar.colour))) fail(`\`${prefix}scrollBar.colour\` must be a hex colour such as "#8a8a8a", got ${JSON.stringify(theme.scrollBar.colour)}`);
  if (theme.scrollBar?.track !== undefined && typeof theme.scrollBar.track !== "boolean") fail(`\`${prefix}scrollBar.track\` must be true or false`);

  const describeInputStyle = (style = {}, path = "textInput") => Object.fromEntries(Object.entries(style).map(([key, value]) => {
    const location = `${path}.${key}`;
    if (["focused", "invalid", "disabled", "readOnly"].includes(key)) return [key, describeInputStyle(value, location)];
    if (key === "border") return [key, choice(value, location, ["underline", "outline", "none"])];
    if (key === "font") return [key, choice(value, location, ["body", "display", "input"])];
    if (key === "align") return [key, choice(value, location, ["start", "center", "end"])];
    if (key === "background" || key.endsWith("Colour")) {
      if (!["foreground", "background", "muted"].includes(value) && (typeof value !== "string" || !colourPattern.test(value))) fail(`\`${prefix}${location}\` must be a hex colour or foreground, background or muted`);
      return [key, value];
    }
    return [key, number(value, location, { min: ["fontSize", "clearSize"].includes(key) ? 1 : 0, max: 10000 })];
  }));
  const hitSlop = theme.hitSlop ?? 0;
  if (typeof hitSlop !== "number" && (typeof hitSlop !== "object" || Array.isArray(hitSlop))) fail("`hitSlop` must be a number or an object of top, right, bottom and left");
  if (typeof hitSlop === "object") check(hitSlop, { top: null, right: null, bottom: null, left: null }, `${prefix}hitSlop.`);
  const hitInsets = Object.fromEntries(["top", "right", "bottom", "left"].map(side => [side,
    number(typeof hitSlop === "number" ? hitSlop : hitSlop[side] ?? 0, `hitSlop.${side}`, { min: -10000, max: 10000 })]));

  return {
    fonts,
    colours: { dark, light },
    text: {
      size: number(theme.text?.size, "text.size", { min: 8, max: 300 }),
    },
    spacing: {
      inset: number(theme.spacing?.inset, "spacing.inset", { max: 400 }),
      gap: number(theme.spacing?.gap, "spacing.gap", { max: 600 }),
      rowGap: number(theme.spacing?.rowGap, "spacing.rowGap", { max: 600 }),
    },
    header: {
      layout: choice(theme.header?.layout, "header.layout", ["centred", "large"]),
      size: number(theme.header?.size, "header.size", { min: 8, max: 300 }),
      top: number(theme.header?.top, "header.top", { max: 600 }),
      bottom: number(theme.header?.bottom, "header.bottom", { max: 600 }),
      cutout: choice(theme.header?.cutout, "header.cutout", ["beside", "below"]),
      back: theme.header?.back && {
        position: choice(theme.header.back.position, "header.back.position", ["start", "end"]),
        icon: choice(theme.header.back.icon, "header.back.icon", ["chevron", "arrow"]),
      },
      iconSize: number(theme.header?.iconSize, "header.iconSize", { min: 8, max: 300 }),
    },
    section: {
      uppercase: theme.section?.uppercase,
      size: number(theme.section?.size, "section.size", { min: 8, max: 300 }),
      gap: number(theme.section?.gap, "section.gap", { max: 600 }),
    },
    scrollBar: {
      colour: theme.scrollBar?.colour,
      visibility: choice(theme.scrollBar?.visibility, "scrollBar.visibility", ["always", "scrolling"]),
      track: theme.scrollBar?.track,
      width: number(theme.scrollBar?.width, "scrollBar.width", { min: 1, max: 100 }),
      inset: number(theme.scrollBar?.inset, "scrollBar.inset", { max: 400 }),
    },
    subtitle: {
      size: number(theme.subtitle?.size, "subtitle.size", { min: 8, max: 300 }),
      tone: choice(theme.subtitle?.tone, "subtitle.tone", ["foreground", "muted"]),
    },
    row: {
      size: number(theme.row?.size, "row.size", { min: 8, max: 300 }),
    },
    toggle,
    selection: choice(theme.selection, "selection", ["underline", "check"]),
    field: choice(theme.field, "field", ["stacked", "inline"]),
    textInput: describeInputStyle(theme.textInput),
    hitSlop: hitInsets,
  };
}

const directory = dirname(themePath);
const description = await describe(base);
description.variants = {};
for (const [name, variant] of Object.entries(variants)) {
  prefix = `variants.${name}.`;
  description.variants[name] = await describe(merge(base, variant));
}
console.log(JSON.stringify(description));
