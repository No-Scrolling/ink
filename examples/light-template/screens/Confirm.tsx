import { back, Confirmation } from "ink";

export default function Confirm() {
  return (
    <Confirmation title="Confirmation" confirmLabel="Confirm" onConfirm={back}>
      Are you sure you want to confirm this example?
    </Confirmation>
  );
}
