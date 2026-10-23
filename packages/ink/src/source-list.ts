import { createElement, useMemo } from "./react";
import { sourceViewId, type SourceTextField, type SourceView } from "./source";

export function SourceList<T>({ view, title, subtitle, image, titleMaxLines = 2, gap = 47, onPress, onLongPress }: {
  view: SourceView<T>;
  title: SourceTextField<T>;
  subtitle?: SourceTextField<T>;
  image?: SourceTextField<T>;
  titleMaxLines?: number;
  gap?: number;
  onPress?: (key: string) => void;
  onLongPress?: (key: string) => void;
}) {
  if (!Number.isFinite(gap) || gap < 0) throw new Error("SourceList gap must be finite and non-negative");
  const press = !!onPress || !!onLongPress;
  const longPress = !!onLongPress;
  const template = useMemo(() => {
    const field = (name: string) => ({ $value: 0, path: [name] });
    return JSON.stringify({ id: 0, type: "Stack", props: { gap: 0, align: "stretch" }, children: [{
      id: 1, type: "RowContent", props: {
        title: field(title), titleMaxLines,
        ...(subtitle === undefined ? {} : { subtitle: field(subtitle) }),
        ...(image === undefined ? {} : { image: field(image) }),
        ...(press ? { onPress: true } : {}),
        ...(longPress ? { onLongPress: true } : {}),
      }, children: [],
    }] });
  }, [title, subtitle, image, titleMaxLines, press, longPress]);
  const props = {
    key: template, sourceView: sourceViewId(view), template, gap,
    onRowEvent(key: string, _target: number, event: string) {
      if (event === "onPress") onPress?.(key);
      if (event === "onLongPress") onLongPress?.(key);
    },
  };
  return createElement("NativeList", props);
}
