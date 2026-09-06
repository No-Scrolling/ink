import { useEffect, useState } from "react";
import { Button, Confirmation, LoadingState, Screen, Text } from "ink";

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
      <Screen title={title} centered>
        <LoadingState label={loadingMessage} align="center" />
      </Screen>
    );
  }

  if (status === "error") {
    return (
      <Confirmation title={title} centered confirmLabel="Try again" onConfirm={() => {
        setRetrying(true);
        setStatus("loading");
      }}>
        The example could not load.
      </Confirmation>
    );
  }

  return (
    <Screen title={title}>
      <Text>The example loaded successfully.</Text>
    </Screen>
  );
}
