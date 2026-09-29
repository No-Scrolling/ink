import * as v from "valibot";
import { Icon, Image, Stack, Text } from "ink";
import { arrowForward } from "ink/icons";
import { callNative } from "ink/native";
import { Barcode, supportedFormats } from "@ink/barcode";

const artworkSchema = v.object({ src: v.string(), width: v.number(), height: v.number() });
export const walletPassSchema = v.object({
  id: v.string(),
  serialNumber: v.string(),
  passTypeIdentifier: v.string(),
  title: v.string(),
  organisation: v.string(),
  description: v.string(),
  artwork: v.optional(v.object({
    logo: v.optional(artworkSchema), strip: v.optional(artworkSchema), thumbnail: v.optional(artworkSchema),
    background: v.optional(artworkSchema), footer: v.optional(artworkSchema), icon: v.optional(artworkSchema),
  })),
  style: v.optional(v.string()),
  fields: v.array(v.object({ label: v.string(), value: v.string(), group: v.optional(v.string()), displayValue: v.optional(v.string()) })),
  barcode: v.optional(v.object({ format: v.picklist(supportedFormats), value: v.string(), altText: v.optional(v.string()) })),
});
const detailFieldsSchema = v.array(v.object({
  label: v.string(),
  parts: v.array(v.object({ text: v.string(), url: v.optional(v.string()) })),
}));
export type PassDetailField = v.InferOutput<typeof detailFieldsSchema>[number];
export type WalletPass = v.InferOutput<typeof walletPassSchema>;

export function isWalletPass(file: { name?: string; mimeType?: string; src?: string }) {
  return file.mimeType?.split(";")[0].trim().toLowerCase() === "application/vnd.apple.pkpass"
    || /\.pkpass(?:[?#]|$)/i.test(file.name || "")
    || /\.pkpass(?:[?#]|$)/i.test(file.src || "");
}

export const wallet = {
  async preview(source: string, signal?: AbortSignal): Promise<WalletPass> {
    return v.parse(walletPassSchema, JSON.parse(await callNative("files", "pass-preview", { source }, { signal })));
  },
  async details(pass: WalletPass): Promise<PassDetailField[]> {
    return v.parse(detailFieldsSchema, JSON.parse(await callNative("files", "pass-details", { pass })));
  },
  async retain(pass: WalletPass): Promise<WalletPass> {
    return v.parse(walletPassSchema, JSON.parse(await callNative("files", "pass-retain", { pass })));
  },
  async release(pass: WalletPass): Promise<void> {
    for (const image of Object.values(pass.artwork || {})) {
      if (image?.src.startsWith(`ink-file://${pass.id}-s-`))
        await callNative("files", "remove", { id: image.src.slice("ink-file://".length) });
    }
  },
  async canOpen(packageName: string): Promise<boolean> {
    return await callNative("files", "pass-can-open", { packageName }) === "true";
  },
  async open(pass: WalletPass, packageName: string): Promise<void> {
    await callNative("files", "pass-open", { id: pass.id, packageName });
  },
};

function Artwork({ image, width = 300, fillWidth = false }: { image?: v.InferOutput<typeof artworkSchema>; width?: number; fillWidth?: boolean }) {
  if (!image) return null;
  if (fillWidth) return <Image src={image.src} width={image.width} height={image.height} fillWidth fit="contain" />;
  width = Math.min(width, image.width);
  const height = Math.min(200, width * image.height / image.width);
  return <Image src={image.src} width={width} height={height} fit="contain" />;
}

type PassField = WalletPass["fields"][number];
function Field({ field, primary = false, paired = false, end = false }: { field: PassField; primary?: boolean; paired?: boolean; end?: boolean }) {
  return <Stack gap={0} align={end ? "end" : "start"}>
    {field.label && <Text size={11} width={primary && paired ? 140 : undefined} align={end ? "end" : "start"}>{field.label.toUpperCase()}</Text>}
    <Text size={primary ? 36 : 16} width={primary && paired ? 140 : undefined} align={end ? "end" : "start"}>{field.displayValue ?? field.value}</Text>
  </Stack>;
}
function FieldRows({ fields, primary = false, transit = false }: { fields: PassField[]; primary?: boolean; transit?: boolean }) {
  const rows = [];
  for (let index = 0; index < fields.length; index += 2) {
    rows.push(<Stack key={index} axis="horizontal" justify="space-between" align="center" gap={6}>
      <Field field={fields[index]} primary={primary} paired={!!fields[index + 1]} />
      {transit && index === 0 && fields[index + 1] && <Icon name={arrowForward} size={24} />}
      {fields[index + 1] && <Field field={fields[index + 1]} primary={primary} paired end />}
    </Stack>);
  }
  return <Stack gap={14} align="stretch">{rows}</Stack>;
}
function routeText(value: string) {
  return value.replace(/\s+/g, "").replace(/[–—→]/g, "-").toUpperCase();
}

export function PassPreview({ pass }: { pass: WalletPass }) {
  const artwork = pass.artwork;
  const fields = pass.fields.filter(field => field.label.trim() || field.value.trim());
  const primary = fields.filter(field => field.group === "primaryFields");
  const secondary = fields.filter(field => field.group === "secondaryFields" || !field.group);
  const route = pass.style === "boardingPass" && primary.length === 2
    ? routeText(`${primary[0].value}-${primary[1].value}`) : undefined;
  const headers = fields.filter(field => field.group === "headerFields"
    && !(route && !field.label.trim() && routeText(field.value.replace(/^[^:]*:/, "")) === route));
  const auxiliary = fields.filter(field => field.group === "auxiliaryFields");
  return <Stack gap={14} align="stretch" justify="space-between">
    <Stack gap={14} align="stretch">
      <Artwork image={artwork?.strip || artwork?.background} fillWidth />
      <Artwork image={artwork?.thumbnail} width={80} />
      {primary.length > 0 && <FieldRows fields={primary} primary transit={pass.style === "boardingPass"} />}
      {(pass.style === "boardingPass" ? [auxiliary, secondary] : [secondary, auxiliary]).map((fields, index) =>
        fields.length > 0 && <FieldRows key={index} fields={fields} />)}
      {headers.length > 0 && <FieldRows fields={headers} />}
      <Artwork image={artwork?.footer} />
    </Stack>
    {pass.barcode ? <Stack align="center" gap={5}>
      <Barcode format={pass.barcode.format} value={pass.barcode.value} size={200} />
      {pass.barcode.altText && <Text size={14} align="center">{pass.barcode.altText}</Text>}
    </Stack> : <Text>This pass has no supported barcode.</Text>}
  </Stack>;
}
