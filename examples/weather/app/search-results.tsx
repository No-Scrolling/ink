import { ErrorState, List, LoadingState, navigate, Row, Screen, useRouteParams } from "ink";
import { useEffect, useState } from "react";
import { searchLocations, type GeocodingResult } from "../lib/weather";
import { formatLocationName } from "../lib/preferences";
import { routeQuery } from "../lib/routeParams";

export default function SearchResultsScreen() {
  const params = useRouteParams<{ query?: string }>();
  const query = routeQuery(params.query).trim();
  const [results, setResults] = useState<GeocodingResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);
    if (!query) {
      setLoading(false);
      setError("Enter a location to search.");
      return () => { active = false; };
    }
    void searchLocations(query).then(value => {
      if (!active) return;
      setResults(value);
      if (value.length === 0) setError("No locations found.");
      setLoading(false);
    }).catch(cause => {
      if (!active) return;
      setError(cause instanceof Error ? cause.message : "Search failed.");
      setLoading(false);
    });
    return () => { active = false; };
  }, [attempt, query]);

  const title = `Results for “${query}”`;
  if (loading) return <Screen title={title}><LoadingState label="Searching…" /></Screen>;
  if (error) return <Screen title={title}><ErrorState message={error} onRetry={() => setAttempt(value => value + 1)} /></Screen>;
  return (
    <Screen title={title}>
      <List
        items={results}
        keyExtractor={result => String(result.id)}
        renderItem={result => (
          <Row title={formatLocationName({ name: result.name, admin1: result.admin1, country: result.country })} onPress={() => navigate({ path: "/weather", params: {
            id: result.id,
            name: result.name,
            admin1: result.admin1 ?? "",
            country: result.country,
            latitude: result.latitude,
            longitude: result.longitude,
          } })} />
        )}
      />
    </Screen>
  );
}
