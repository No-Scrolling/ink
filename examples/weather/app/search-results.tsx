import { ErrorState, List, LoadingState, navigate, Row, Screen, resource, useRouteParams, useSnapshot } from "ink";
import { searchLocations } from "../lib/weather";
import { formatLocationName } from "../lib/preferences";
import { routeQuery } from "../lib/routeParams";

const searches = resource({
  key: (query: string) => [query],
  load: async query => {
    if (!query) throw new Error("Enter a location to search.");
    return searchLocations(query);
  },
  staleTime: 300_000,
});

export default function SearchResultsScreen() {
  const params = useRouteParams<{ query?: string }>();
  const query = routeQuery(params.query).trim();
  const source = searches(query);
  const result = useSnapshot(source);
  const title = `Results for “${query}”`;
  if (result.status === "loading") return <Screen title={title}><LoadingState label="Searching…" /></Screen>;
  if (result.status === "error") return <Screen title={title}><ErrorState message={result.error.message} onRetry={source.refresh} /></Screen>;
  if (!result.data.length) return <Screen title={title}><ErrorState message="No locations found." onRetry={source.refresh} /></Screen>;
  return (
    <Screen title={title}>
      <List
        items={result.data}
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
