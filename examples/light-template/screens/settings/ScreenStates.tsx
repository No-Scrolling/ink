import { useEffect, useState } from "react";
import { Button, ErrorState, LoadingState, Screen, Text } from "ink";

type State = { status: "idle" | "error" | "ready" } | { status: "loading"; result: "error" | "ready" };

export default function ScreenStates() {
  const [state, setState] = useState<State>({ status: "idle" });
  useEffect(() => {
    if (state.status !== "loading") return;
    const timer = setTimeout(() => setState({ status: state.result }), 1500);
    return () => clearTimeout(timer);
  }, [state]);
  function load(result: "error" | "ready") {
    setState({ status: "loading", result });
  }
  return (
    <Screen title="Screen states">
      <Text size={18}>A local demonstration of loading, errors and retry. No network request is made.</Text>
      {state.status === "idle" && <Button onPress={() => load("error")}>Start demonstration</Button>}
      {state.status === "loading" && <LoadingState label="Loading example…" />}
      {state.status === "error" && <ErrorState message="The example could not load. Try again to complete the demonstration."
        onRetry={() => load("ready")} />}
      {state.status === "ready" && <>
        <Text size={18}>The example loaded successfully.</Text>
        <Button onPress={() => setState({ status: "idle" })}>Restart demonstration</Button>
      </>}
    </Screen>
  );
}
