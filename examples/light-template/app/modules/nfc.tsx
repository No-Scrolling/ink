import { useEffect, useRef } from "react";
import { nfc } from "@ink/nfc";
import { Button, Field, Screen, Stack, useAction } from "ink";

export default function Nfc() {
  const session = useRef<AbortController | null>(null);
  const read = () => {
    const controller = new AbortController();
    session.current = controller;
    return nfc.read({ signal: controller.signal });
  };
  const tag = useAction(read);
  const raw = useAction(async (technology: "iso-dep" | "nfc-a") => {
    session.current?.abort();
    const controller = new AbortController();
    session.current = controller;
    const connection = await nfc.connect({ technology, signal: controller.signal });
    try {
      return `${connection.technology}: ${connection.serialNumber} (${connection.maxTransceiveLength} bytes)`;
    } finally {
      await connection.close();
    }
  });
  const card = useAction(async (enabled: boolean) => {
    session.current?.abort();
    if (enabled) {
      await nfc.emulate({
        aids: ["F000000001"],
        responses: [{ command: "00A4040005F000000001", response: "9000" }],
      });
    } else await nfc.stopEmulation();
    return enabled ? "Enabled" : "Disabled";
  });
  useEffect(() => {
    tag.run();
    return () => {
      session.current?.abort();
      session.current = null;
    };
  }, [tag.run]);

  return (
    <Screen title="NFC">
      {tag.status === "success" ? (
        <Stack gap={16}>
          <Field label="Serial number">{tag.data.serialNumber}</Field>
          <Field label="Text">{tag.data.hasText ? tag.data.text : "None"}</Field>
          <Field label="URI">{tag.data.hasUri ? tag.data.uri : "None"}</Field>
        </Stack>
      ) : (
        <Field label="Tag">
          {tag.status === "error" ? tag.error.message : "Hold an NFC tag near the phone..."}
        </Field>
      )}
      <Button
        onPress={() => {
          if (raw.status !== "pending") tag.run();
        }}
      >
        Read another tag
      </Button>
      <Button onPress={() => raw.run("iso-dep")}>Read ISO-DEP Details</Button>
      <Button onPress={() => raw.run("nfc-a")}>Read NFC-A Details</Button>
      {raw.status === "pending" && (
        <Field label="Connection">Hold a compatible tag near the phone...</Field>
      )}
      {raw.status === "success" && <Field label="Connection">{raw.data}</Field>}
      {raw.status === "error" && <Field label="Connection">{raw.error.message}</Field>}
      <Button onPress={() => card.run(true)}>Enable Demo Card</Button>
      <Button onPress={() => card.run(false)}>Disable Demo Card</Button>
      {card.status === "success" && <Field label="Demo card">{card.data}</Field>}
      {card.status === "error" && <Field label="Demo card">{card.error.message}</Field>}
    </Screen>
  );
}
