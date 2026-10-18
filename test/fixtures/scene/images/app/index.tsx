import { Image, Screen } from "ink";

// The first image fills the viewport; the rest sit below it, inside the scroll overscan.
export default function App() {
  return (
    <Screen title="Images">
      <Image src="https://example.com/visible.png" width={300} height={400} />
      {["below-1", "below-2", "below-3"].map((name) => (
        <Image key={name} src={`https://example.com/${name}.png`} width={300} height={100} />
      ))}
    </Screen>
  );
}
