import { List, navigate, Row, Screen, Text } from "ink";
import { add } from "ink/icons";
import { PreferencesGate } from "../../components/PreferencesGate";
import { formatLocationName, type WeatherPreferences } from "../../lib/preferences";

export default function LocationsScreen() {
  return <PreferencesGate title="Locations">{prefs => <Locations prefs={prefs} />}</PreferencesGate>;
}

function Locations({ prefs }: { prefs: WeatherPreferences }) {
  return (
    <Screen title="Locations" rightAction={{ icon: add, onPress: () => navigate("/search") }}>
      {prefs.savedLocations.length === 0 && <Text size={18}>No saved locations.</Text>}
      <List
        items={prefs.savedLocations}
        keyExtractor={location => `${location.id}:${location.latitude}:${location.longitude}`}
        renderItem={saved => <Row title={formatLocationName(saved)} onPress={() => navigate({ path: "/weather", params: {
          id: saved.id,
          name: saved.name,
          admin1: saved.admin1 ?? "",
          country: saved.country,
          latitude: saved.latitude,
          longitude: saved.longitude,
        } })} />}
      />
    </Screen>
  );
}
