import { ErrorState, LoadingState, Screen, useRouteParams, useSnapshot } from "ink";
import { routeResource } from "../../data";
import { JourneyRows } from "../../ui";

export default function Detail() {
  const { id } = useRouteParams("/detail/[id]");
  const source = routeResource(id);
  const snapshot = useSnapshot(source);
  return <Screen title="Route library" wide>
    {snapshot.status === "loading" ? <LoadingState label={`Loading route ${id}`} /> :
      snapshot.status === "error" ? <ErrorState message={`Route ${id} failed`} retryLabel="Retry route"
        onRetry={() => source.refresh()} /> : <JourneyRows items={snapshot.data} />}
  </Screen>;
}
