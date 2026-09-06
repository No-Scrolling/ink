/** Reference from an imported .ink-icons collection. */
export type IconAsset = string & { readonly __inkIcon: unique symbol };

/** Unknown names in external data return undefined. */
export function findIcon(collection: Readonly<Record<string, IconAsset>>, name: string): IconAsset | undefined {
  return Object.hasOwn(collection, name) ? collection[name] : undefined;
}
