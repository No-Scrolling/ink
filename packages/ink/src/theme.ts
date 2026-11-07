/**
 * App themes. Export a theme from `theme.ts` in the app directory:
 *
 * ```ts
 * import { defineTheme } from "ink/theme";
 *
 * export default defineTheme({
 *   fonts: { body: "./assets/fonts/Inter-Regular.ttf" },
 *   colours: { foreground: "#ffffff", muted: "#8a8a8a" },
 *   text: { size: 44 },
 * });
 * ```
 *
 * Ink evaluates the theme when it builds the app. Sizes are pixels in the 1080 × 1240 export
 * frame, so values measured from an `ink export` screenshot or a mockup at that size can be used
 * directly.
 */

/** A hex colour: `#rgb`, `#rrggbb` or `#rrggbbaa`. */
export type Colour = `#${string}`;

export type InputColour = Colour | "foreground" | "background" | "muted";

export interface TextInputStyle {
  border?: "underline" | "outline" | "none";
  borderWidth?: number;
  borderColour?: InputColour;
  radius?: number;
  background?: InputColour;
  paddingX?: number;
  paddingY?: number;
  minHeight?: number;
  fontSize?: number;
  /** Choose a font from `fonts`. Supply a font file with the required weight. */
  font?: "body" | "display" | "input";
  align?: "start" | "center" | "end";
  textColour?: InputColour;
  placeholderColour?: InputColour;
  affixColour?: InputColour;
  cursorColour?: InputColour;
  selectionColour?: InputColour;
  clearColour?: InputColour;
  clearSize?: number;
  clearPadding?: number;
  clearGap?: number;
}

export interface TextInputAppearance extends TextInputStyle {
  focused?: TextInputStyle;
  invalid?: TextInputStyle;
  disabled?: TextInputStyle;
  readOnly?: TextInputStyle;
}

export interface Colours {
  /** Screen background. Default: `#000000`. */
  background?: Colour;
  /** Text, icons and controls. Default: `#ffffff`. */
  foreground?: Colour;
  /** Secondary text, values and inactive controls. Default: `#898989`. */
  muted?: Colour;
}

export interface ThemeColours extends Colours {
  /** Light mode colours. Each role defaults to the inverse of its dark colour. */
  light?: Colours;
}

/** Colours passed to a custom control, resolved for the scheme being drawn. */
export interface ResolvedColours {
  background: string;
  foreground: string;
  muted: string;
}

export interface ToggleState {
  on: boolean;
  /** The toggle cannot be changed. Draw it dimmed, for example with lower opacity. */
  disabled: boolean;
  /** `"dark"` or `"light"`. */
  scheme: "dark" | "light";
  colours: ResolvedColours;
}

export interface ToggleControl {
  readonly kind: "toggle";
  /** Width in pixels. The label fills the rest of the row. */
  width: number;
  /** Height in pixels. */
  height: number;
  /** Returns SVG markup drawn into a `width` × `height` box. Ink renders each state once at build time. */
  render: (state: ToggleState) => string;
}

export interface Theme {
  /** Font files, relative to `theme.ts`. `display` is used for screen titles and defaults to `body`. */
  fonts?: { body?: string; display?: string; input?: string };
  /** Dark mode colours, with optional `light` overrides. */
  colours?: ThemeColours;
  text?: {
    /**
     * Body text size in pixels. Default: 76.5.
     *
     * This sets the size of Ink's units: body text is 30 units, and every size in the app scales with it,
     * including `Text` `size`, `Stack` `gap`, `Icon` `size` and Canvas coordinates. `<Text size={40}>` stays
     * 4/3 of body text under any theme.
     */
    size?: number;
  };
  spacing?: {
    /** Horizontal screen padding in pixels. Default: 94. */
    inset?: number;
    /** Gap between screen items and section rows, in pixels. Default: 120 at the default text size. */
    gap?: number;
  };
  header?: {
    /** `"centred"`: a small centred title (default). `"large"`: a large title aligned with the content, with an optional subtitle. */
    layout?: "centred" | "large";
    /** Title size in pixels. */
    size?: number;
    /** Space above the title row, in pixels. Default: 36 for `"large"`; for `"centred"`, half of what fills Ink's header bar. */
    top?: number;
    /** Space between the title row (or subtitle) and the content, in pixels. Default: 56 for `"large"`; for `"centred"`, the same as `top`. */
    bottom?: number;
    back?: {
      /** `"start"` (default) or `"end"`. At the end, Back takes the end slot and the screen's end action moves in beside it. */
      position?: "start" | "end";
      /** `"chevron"` (default) or `"arrow"`. */
      icon?: "chevron" | "arrow";
    };
    /** Size of Back and header action icons in pixels, like `Icon` `size`. Material Symbols leave space around the drawn shape, so an arrow drawn 34 pixels wide needs about 60. */
    iconSize?: number;
    /** Camera cutouts at the top of the screen. `"beside"`: the header fits in the widest space beside them (default). `"below"`: the screen starts below them. */
    cutout?: "beside" | "below";
  };
  section?: {
    /** Show section titles in capitals. Default: `true`. */
    uppercase?: boolean;
    /** Section title size in pixels. */
    size?: number;
    /** Space between a section title and its first row, in pixels. Default: half of `spacing.gap`. */
    gap?: number;
  };
  /** Secondary lines: the large header's subtitle, and toggle and row subtitles. */
  subtitle?: {
    /** Size in pixels. Default: about 48 for header subtitles and 41 for toggle and row subtitles. */
    size?: number;
    /** Default: `"muted"` for header subtitles, `"foreground"` for toggle and row subtitles. */
    tone?: "foreground" | "muted";
  };
  scrollBar?: {
    /** Colour of the thumb and track. Default: the foreground. Light mode uses its inverse. */
    colour?: Colour;
    /** `"always"` (default) or `"scrolling"`: shown while the screen scrolls, then faded out. */
    visibility?: "always" | "scrolling";
    /** Draw the thin full-height line the thumb moves along. Default: `true`. */
    track?: boolean;
    /** Thumb width in pixels. Default: about 13 at the default text size. */
    width?: number;
    /** Distance from the screen edge to the bar, in pixels. Setting it keeps the content's own end padding. */
    inset?: number;
  };
  /** `"line"`: Ink's line and circle (default). `"switch"`: a sliding switch after the label. Or a custom control from `defineToggle`. */
  toggle?: "line" | "switch" | ToggleControl;
  /** How a selected `Button` is marked. `"underline"` (default) or `"check"`: a tick at the end of the row. */
  selection?: "underline" | "check";
  /** `"stacked"`: label above value (default). `"inline"`: label and value on one row, with a chevron when pressable. */
  field?: "stacked" | "inline";
  /** Input appearance. Sizes are export-frame pixels, including state overrides. */
  textInput?: TextInputAppearance;
  /** Extra touch area around controls, in export-frame pixels. Negative values shrink it. */
  hitSlop?: import("./press").HitSlop;
  /**
   * Named themes for individual screens, used with `<Screen theme="name">`. Each variant changes
   * only the settings it sets; everything else comes from this theme. Ink prepares every variant
   * when it builds the app and loads all their fonts at startup.
   */
  variants?: Record<string, Omit<Theme, "variants">>;
}

export function defineTheme<const T extends Theme>(theme: T): T {
  return theme;
}

export function defineToggle(control: Omit<ToggleControl, "kind">): ToggleControl {
  return { kind: "toggle", ...control };
}
