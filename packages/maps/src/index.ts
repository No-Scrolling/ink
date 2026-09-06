import { createElement, useEffect, useMemo, useRef, useState } from "react";
import { Field, useColourScheme } from "ink";
import { NativeError } from "ink/native";
import { attachNativeController } from "ink/native/controller";

export interface Coordinate { latitude: number; longitude: number }
export interface MapCamera { centre: Coordinate; zoom: number }
export interface MapMarker extends Coordinate { id: string; label?: string }
type Attachment = ReturnType<typeof attachNativeController>;
const attachmentKey = Symbol("map attachment");
export interface MapController {
  moveTo(camera: MapCamera): Promise<void>;
  readonly [attachmentKey]: { current: Attachment | null };
}

function coordinate(value: Coordinate): Coordinate {
  if (!Number.isFinite(value.latitude) || Math.abs(value.latitude) > 90
    || !Number.isFinite(value.longitude) || Math.abs(value.longitude) > 180) {
    throw new RangeError("Map coordinates require latitude between -90 and 90 and longitude between -180 and 180");
  }
  return { latitude: value.latitude, longitude: value.longitude };
}
function zoomLevel(value: number): number {
  if (!Number.isFinite(value) || value < 0 || value > 22) throw new RangeError("Map zoom must be between 0 and 22");
  return value;
}
export function useMap(): MapController {
  const attachment = useRef<Attachment | null>(null);
  return useMemo(() => ({
    [attachmentKey]: attachment,
    async moveTo({ centre, zoom }: MapCamera) {
      const camera = { centre: coordinate(centre), zoom: zoomLevel(zoom) };
      if (!attachment.current) throw new NativeError("unavailable", "Map is not attached");
      await attachment.current.call("move-to", camera);
    },
  }), []);
}

export interface MapViewProps {
  styleURL?: string;
  initialCentre: Coordinate;
  initialZoom?: number;
  markers?: readonly MapMarker[];
  controller?: MapController;
  onMarkerPress?: (id: string) => void;
  onCameraIdle?: (camera: MapCamera) => void;
}
export function MapView({ styleURL: customStyleURL, initialCentre, initialZoom = 14, markers = [], controller,
  onMarkerPress, onCameraIdle }: MapViewProps) {
  const colourScheme = useColourScheme();
  const styleURL = customStyleURL ?? `https://tiles.openfreemap.org/styles/${colourScheme === "light" ? "positron" : "dark"}`;
  const ownController = useMap();
  const mapController = controller ?? ownController;
  const initial = useRef({ initialCentre: coordinate(initialCentre), initialZoom: zoomLevel(initialZoom) });
  const callbacks = useRef({ onMarkerPress, onCameraIdle });
  callbacks.current = { onMarkerPress, onCameraIdle };
  const [id, setId] = useState<number | null>(null);
  const [error, setError] = useState<Error | null>(null);
  if (!styleURL.startsWith("https://")) throw new TypeError("Map styleURL must use HTTPS");
  const ids = new Set<string>();
  const update = JSON.stringify({ styleURL, colourScheme, markers: markers.map(marker => {
    if (!marker.id || ids.has(marker.id)) throw new TypeError("Map markers require unique non-empty IDs");
    ids.add(marker.id);
    return { id: marker.id, ...coordinate(marker), ...(marker.label === undefined ? {} : { label: marker.label }) };
  }) });
  const latestUpdate = useRef(update);
  latestUpdate.current = update;
  useEffect(() => {
    if (mapController[attachmentKey].current) throw new Error("A map controller can only belong to one MapView");
    setError(null);
    setId(null);
    let active = true;
    const attachment = attachNativeController("maps", { ...initial.current, ...JSON.parse(latestUpdate.current) }, value => {
      if (!active) return;
      if (typeof value !== "object" || value === null || !("type" in value)) {
        setError(new NativeError("protocol", "Invalid map event"));
      } else if (value.type === "ready") {
        setError(null);
      } else if (value.type === "marker-press" && "id" in value && typeof value.id === "string") {
        callbacks.current.onMarkerPress?.(value.id);
      } else if (value.type === "camera-idle" && "centre" in value && typeof value.centre === "object"
        && value.centre !== null && "latitude" in value.centre && typeof value.centre.latitude === "number"
        && "longitude" in value.centre && typeof value.centre.longitude === "number"
        && "zoom" in value && typeof value.zoom === "number") {
        const camera = { centre: { latitude: value.centre.latitude, longitude: value.centre.longitude }, zoom: value.zoom };
        initial.current = { initialCentre: camera.centre, initialZoom: camera.zoom };
        callbacks.current.onCameraIdle?.(camera);
      } else if (value.type === "error" && "error" in value && typeof value.error === "object" && value.error !== null
        && "message" in value.error && typeof value.error.message === "string") {
        setError(new NativeError("unexpected", value.error.message));
      } else setError(new NativeError("protocol", "Invalid map event"));
    });
    mapController[attachmentKey].current = attachment;
    void attachment.ready.then(() => { if (active) setId(attachment.id); }, failure => { if (active) setError(failure); });
    return () => {
      active = false;
      mapController[attachmentKey].current = null;
      void attachment.dispose().catch(failure => console.error("Could not release map", failure));
    };
  }, [mapController]);
  useEffect(() => { setError(null); }, [styleURL]);
  useEffect(() => {
    let active = true;
    const attachment = mapController[attachmentKey].current;
    if (attachment) void attachment.call("update", JSON.parse(update)).catch(failure => { if (active) setError(failure); });
    return () => { active = false; };
  }, [mapController, update]);
  if (error) return createElement(Field, { label: "Map" }, error.message);
  if (id === null) return null;
  return createElement("MapView", { controller: id });
}
