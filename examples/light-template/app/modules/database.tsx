import { openDatabase } from "@ink/store";
import { expression, nativeView } from "ink/view";
import database from "../../assets/places.db";

type Place = { id: string; name: string };

export default nativeView(view => {
  const places = view.collection<Place>([], "id");
  const error = view.value("");
  const noError = view.compute(expression.equal(error, ""));
  let signal: AbortSignal;
  view.onMount(() => {
    const controller = new AbortController();
    signal = controller.signal;
    return () => controller.abort();
  });
  const search = async (active: AbortSignal) => {
    error.set("");
    const db = await openDatabase(database);
    try {
      if (!active.aborted) await db.queryInto(places,
        "SELECT CAST(id AS TEXT) AS id, name FROM places WHERE name LIKE ? ORDER BY name LIMIT 10",
        ["%Station%"], { signal: active });
    } finally { await db.close(); }
  };
  return view.node("Screen", { title: "Database" },
    view.node("Button", { onPress: () => {
      const active = signal;
      void search(active).catch(failure => {
        if (!active.aborted) error.set(failure instanceof Error ? failure.message : String(failure));
      });
    } }, view.node("Text", { text: "Find stations" })),
    view.list(places, { key: "id" }, place => view.node("Text", { text: place.at("name") })),
    view.node("Text", { text: error, hidden: noError }),
  );
});
