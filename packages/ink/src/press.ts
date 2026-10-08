import { navigate, type CheckedDestination, type Destination } from "./navigation";
import { openLink } from "./external";

export type PressProps<D extends Destination = Destination> =
  | { href: D & CheckedDestination<D>; url?: never; onPress?: never }
  | { url: string; href?: never; onPress?: never }
  | { onPress?: () => void; href?: never; url?: never };

export function pressHandler({ href, url, onPress }: { href?: Destination; url?: string; onPress?: () => void }) {
  if (href !== undefined && (url !== undefined || onPress !== undefined) || url !== undefined && onPress !== undefined) {
    throw new Error("Use only one of href, url or onPress");
  }
  if (href !== undefined) return () => navigate(href);
  if (url !== undefined) return () => openLink(url);
  return onPress;
}
