import { useEffect, useState } from "react";
import { Button, ErrorState, LoadingState, Screen, Text } from "ink";

export default function ScreenStates() {
  return (
    <Screen title="Screen states">
      <Button href="/settings/screen-states/loading">Loading</Button>
      <Button href="/settings/screen-states/error">Error</Button>
    </Screen>
  );
}

export function ScreenStateExample({ result, loadingMessage = "Loading..." }: {
  result: "ready" | "error";
  loadingMessage?: string;
}) {
  const [status, setStatus] = useState<"loading" | "error" | "ready">("loading");
  const [retrying, setRetrying] = useState(false);
  const title = result === "error" ? "Error" : "Loading";

  useEffect(() => {
    if (status !== "loading") return;
    const timer = setTimeout(() => setStatus(retrying ? "ready" : result), 1500);
    return () => clearTimeout(timer);
  }, [status, result, retrying]);

  if (status === "loading") {
    return (
      <Screen title={title}>
        <LoadingState label={loadingMessage} />
      </Screen>
    );
  }

  if (status === "error") {
    return (
      <Screen title={title}>
        <ErrorState message="The example could not load." onRetry={() => {
          setRetrying(true);
          setStatus("loading");
        }} />
      </Screen>
    );
  }

  return (
    <Screen title={title}>
      <Text>The example loaded successfully.</Text>
    </Screen>
  );
}
