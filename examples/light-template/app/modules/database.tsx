import { useEffect, useRef, useState } from "react";
import { openDatabase, type SqlRow } from "@ink/store";
import { Button, Screen, Text } from "ink";
import database from "../../assets/places.db";

export default function Database() {
  const [places, setPlaces] = useState<readonly SqlRow[]>([]);
  const [error, setError] = useState("");
  const pending = useRef<AbortController | null>(null);
  useEffect(() => () => pending.current?.abort(), []);

  async function search() {
    pending.current?.abort();
    const controller = new AbortController();
    pending.current = controller;
    setError("");
    try {
      const db = await openDatabase(database);
      try {
        if (controller.signal.aborted) return;
        const rows = await db.query(
          "SELECT CAST(id AS TEXT) AS id, name FROM places WHERE name LIKE ? ORDER BY name LIMIT 10",
          ["%Station%"],
          { signal: controller.signal },
        );
        if (!controller.signal.aborted) setPlaces(rows);
      } finally {
        await db.close();
      }
    } catch (failure) {
      if (!controller.signal.aborted)
        setError(failure instanceof Error ? failure.message : String(failure));
    }
  }

  return (
    <Screen title="Database">
      <Button onPress={() => void search()}>Find stations</Button>
      {places.map((place) => (
        <Text key={String(place.id)}>{place.name}</Text>
      ))}
      {error && <Text>{error}</Text>}
    </Screen>
  );
}
