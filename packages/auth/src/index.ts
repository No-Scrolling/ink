import type { Snapshot, SnapshotSource } from "ink";
import { callNative, onNativeMessage } from "ink/native";

const observers = new Map<string, Set<() => void>>();
onNativeMessage("auth-changed", message => {
  if (typeof message.id === "string") for (const refresh of observers.get(message.id) ?? []) refresh();
});

export interface OAuthConfig {
  id: string;
  clientId: string;
  tokenEndpoint: string;
  scopes?: string[];
  deviceAuthorizationEndpoint?: string;
  authorizationEndpoint?: string;
  redirectUri?: string;
}
export type AccountState = { status: "signed-out" | "signed-in" };
type Options = { signal?: AbortSignal };
type DeviceResponse = { error?: string; device_code: string; user_code: string; verification_uri: string; verification_uri_complete?: string; expires_in: number; interval?: number };
export interface DeviceSignIn {
  verificationUri: string;
  verificationUriComplete?: string;
  userCode: string;
  expiresAt: number;
  complete(options?: Options): Promise<void>;
}
export interface OAuthClient extends SnapshotSource<AccountState> {
  startDeviceSignIn(options?: Options): Promise<DeviceSignIn>;
  signIn(options?: Options): Promise<void>;
  getAccessToken(options?: Options): Promise<string>;
  signOut(): Promise<void>;
}

function delay(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal.aborted) { reject(signal.reason); return; }
    const abort = () => { clearTimeout(timer); reject(signal.reason); };
    const timer = setTimeout(() => { signal.removeEventListener("abort", abort); resolve(); }, ms);
    signal.addEventListener("abort", abort, { once: true });
  });
}

export function createOAuthClient(configuration: OAuthConfig): OAuthClient {
  const config = { ...configuration, scopes: [...(configuration.scopes ?? [])] };
  if (!config.id || !config.clientId || !config.tokenEndpoint) throw new Error("Account ID, client ID and token endpoint are required");
  let snapshot: Snapshot<AccountState> = { status: "loading" };
  const listeners = new Set<() => void>();
  const work = new Set<AbortController>();
  function publish(value: Snapshot<AccountState>) { snapshot = value; for (const listener of listeners) listener(); }
  async function invoke<T = unknown>(operation: string, payload = {}, options: Options = {}): Promise<T> {
    return JSON.parse(await callNative("auth", operation, { config, ...payload }, { ...options, timeoutMs: 120_000 }));
  }
  async function refreshSnapshot() {
    try { publish({ status: "ready", data: await invoke<AccountState>("status") }); }
    catch (error) { publish({ status: "error", error: error instanceof Error ? error : new Error(String(error)) }); }
  }
  async function owned<T>(options: Options, run: (signal: AbortSignal) => Promise<T>): Promise<T> {
    const controller = new AbortController(); work.add(controller);
    const abort = () => controller.abort(options.signal?.reason);
    options.signal?.addEventListener("abort", abort, { once: true });
    if (options.signal?.aborted) abort();
    try { return await run(controller.signal); }
    finally { work.delete(controller); options.signal?.removeEventListener("abort", abort); }
  }
  async function exchange(generation: number, fields: Record<string, string>, signal: AbortSignal) {
    const result = await invoke<{ error?: string }>("exchange", { generation, fields }, { signal });
    if (result.error) return result.error;
    await refreshSnapshot();
    return null;
  }
  return {
    getSnapshot: () => snapshot,
    subscribe(listener) {
      listeners.add(listener);
      let accountObservers = observers.get(config.id);
      if (!accountObservers) { accountObservers = new Set(); observers.set(config.id, accountObservers); }
      accountObservers.add(refreshSnapshot);
      void refreshSnapshot();
      return () => {
        listeners.delete(listener);
        if (!listeners.size) {
          accountObservers.delete(refreshSnapshot);
          if (!accountObservers.size) observers.delete(config.id);
        }
      };
    },
    async getAccessToken(options = {}) {
      const result: unknown = await invoke("token", {}, options);
      if (typeof result !== "string") throw new Error("Token refresh failed; interactive sign-in may be required");
      await refreshSnapshot();
      return result;
    },
    async signOut() {
      for (const controller of work) controller.abort(new Error("Signed out"));
      await invoke("sign-out");
      await refreshSnapshot();
    },
    async signIn(options = {}) {
      if (!config.authorizationEndpoint || !config.redirectUri) throw new Error("Browser sign-in requires authorisation endpoint and redirect URI");
      await owned(options, async signal => {
        const { generation } = await invoke<{ generation: number }>("begin", {}, { signal });
        const fields: Record<string, string> = JSON.parse(await callNative("external", "browser-auth", { config }, { signal, timeoutMs: 900_000 }));
        const error = await exchange(generation, fields, signal);
        if (error) throw new Error(`Sign-in failed: ${error}`);
      });
    },
    async startDeviceSignIn(options = {}) {
      if (!config.deviceAuthorizationEndpoint) throw new Error("Device sign-in requires a device authorisation endpoint");
      const { generation } = await invoke<{ generation: number }>("begin", {}, options);
      const result = await invoke<DeviceResponse>("device-start", {}, options);
      if (result.error) throw new Error(`Device authorisation failed: ${result.error}`);
      if (typeof result.device_code !== "string" || !result.device_code || typeof result.user_code !== "string" || !result.user_code || typeof result.verification_uri !== "string"
        || !Number.isFinite(result.expires_in) || result.expires_in <= 0 || result.expires_in > 31_536_000
        || (result.interval !== undefined && (!Number.isFinite(result.interval) || result.interval <= 0 || result.interval > 86_400))
        || (result.verification_uri_complete !== undefined && typeof result.verification_uri_complete !== "string")) throw new Error("Invalid device authorisation response");
      const expiresAt = Date.now() + result.expires_in * 1000;
      let interval = Math.max(5, result.interval ?? 5) * 1000;
      let completion: Promise<void> | undefined;
      return {
        verificationUri: result.verification_uri,
        verificationUriComplete: result.verification_uri_complete,
        userCode: result.user_code,
        expiresAt,
        complete(completionOptions = {}) {
          if (completion) return completion;
          completion = owned(completionOptions, async signal => {
            while (Date.now() < expiresAt) {
              await delay(Math.min(interval, Math.max(0, expiresAt - Date.now())), signal);
              if (Date.now() >= expiresAt) break;
              const error = await exchange(generation, { grant_type: "urn:ietf:params:oauth:grant-type:device_code", device_code: result.device_code }, signal);
              if (error === null) return;
              if (error === "slow_down") interval += 5000;
              else if (error !== "authorization_pending") throw new Error(`Device sign-in failed: ${error}`);
            }
            throw new Error("Device sign-in expired");
          });
          return completion;
        },
      };
    },
  };
}
