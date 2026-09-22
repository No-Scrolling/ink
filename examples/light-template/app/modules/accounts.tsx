import { useEffect, useRef, useState } from "react";
import { createOAuthClient, type DeviceSignIn } from "@ink/auth";
import { Button, Field, Screen, Text, useAction, useSnapshot } from "ink";

const endpoint = "http://127.0.0.1:8788";
const account = createOAuthClient({
  id: "template", clientId: "ink-template",
  tokenEndpoint: `${endpoint}/token`,
  deviceAuthorizationEndpoint: `${endpoint}/device`,
  authorizationEndpoint: `${endpoint}/authorize`,
  redirectUri: "ink-template://oauth/callback", scopes: ["profile"],
});

export default function Accounts() {
  const session = useSnapshot(account);
  const controller = useRef<AbortController | null>(null);
  const [device, setDevice] = useState<DeviceSignIn | null>(null);
  useEffect(() => {
    const active = new AbortController();
    controller.current = active;
    return () => active.abort();
  }, []);
  const start = useAction(async () => {
    const pending = await account.startDeviceSignIn({ signal: controller.current?.signal });
    setDevice(pending);
    try { await pending.complete({ signal: controller.current?.signal }); }
    finally { setDevice(null); }
  });
  const browser = useAction(() => account.signIn({ signal: controller.current?.signal }));
  const token = useAction(async () => { await account.getAccessToken(); return "Access token ready"; });
  const signOut = useAction(async () => { await account.signOut(); setDevice(null); });
  return <Screen title="Accounts">
    <Field label="Session">{session.status === "ready" ? session.data.status : session.status}</Field>
    <Button onPress={start.run}>Device sign-in</Button>
    <Button onPress={browser.run}>Browser sign-in</Button>
    <Button onPress={token.run}>Get token</Button>
    <Button onPress={signOut.run}>Sign out</Button>
    {device && <>
      <Field label="Code">{device.userCode}</Field>
      <Text>{device.verificationUriComplete ?? device.verificationUri}</Text>
    </>}
    {token.status === "success" && <Text>{token.data}</Text>}
    {start.status === "error" && <Text>{start.error.message}</Text>}
    {browser.status === "error" && <Text>{browser.error.message}</Text>}
    {token.status === "error" && <Text>{token.error.message}</Text>}
    {signOut.status === "error" && <Text>{signOut.error.message}</Text>}
    {session.status === "error" && <Text>{session.error.message}</Text>}
  </Screen>;
}
