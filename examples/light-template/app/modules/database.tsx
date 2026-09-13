import { useState } from "react";
import { openDatabase } from "@ink/store";
import { Button, Screen, Text, useAction } from "ink";
import database from "../../assets/places.db";

export default function Database() {
  const [names, setNames] = useState<readonly string[]>([]);
  const search = useAction(async () => {
    const db = await openDatabase(database);
    try {
      const rows = await db.query("SELECT name FROM places WHERE name LIKE ? ORDER BY name LIMIT 10", ["%Station%"]);
      setNames(rows.map(row => String(row.name)));
    } finally { await db.close(); }
  });
  return <Screen title="Database">
    <Button onPress={() => search.run()}>Find stations</Button>
    {names.map(name => <Text key={name}>{name}</Text>)}
    {search.status === "error" && <Text>{search.error.message}</Text>}
  </Screen>;
}
