import { callNative } from "./native";

export async function openURL(url: string): Promise<void> {
  await callNative("external", "open-url", { url }, { timeoutMs: 900_000 });
}

export function openLink(url: string): void {
  void openURL(url).catch(error => console.error("Could not open link", error));
}

export async function share(options: { text: string }): Promise<void> {
  await callNative("external", "share", options, { timeoutMs: 900_000 });
}
