import { createElement, useSyncExternalStore } from "react";
import { render } from "ink/renderer";
import { overlayComponents, overlayContent } from "./internal";

function OverlayRoot() {
  const content = useSyncExternalStore(overlayContent.subscribe, overlayContent.getSnapshot);
  if (!content) return null;
  const component = overlayComponents.get(content.id);
  if (!component) throw new Error(`Overlay is not registered in the overlay entry point: ${content.id}`);
  return createElement(component, { ...content.props, key: content.id });
}

export function startOverlays() {
  render(<OverlayRoot />);
}
