# Networking and remote images for Ink

Research date: 30 August 2026. Sources are limited to Android documentation and source, official Rust crate documentation and source, and the HTTP specifications.

## Recommendation

Make networking a conditional, framework-owned module exposed from `@ink/network`, and let an HTTPS `src` turn the existing `Image` into a remote image without introducing a second image component. The module should preserve the resource/action distinction already established by Ink:

- `http.json<T>()` and `http.text()` describe screen-owned reads and return typed `AsyncResource`s.
- `http.action<T>()` describes an explicit write and returns a typed, observable action. It should be implemented when Ink needs its first response-bearing write; a fire-and-forget native action is insufficient for message sending.
- `<Image src="https://…">` owns its loading lifecycle internally. Width and height remain required, so network latency never changes layout.

The Android adapter should use the platform `android.net.http.HttpEngine`, not bundle a Rust HTTP and TLS stack. Ink already targets API 34, where `HttpEngine` is available. It is asynchronous and cancellable, selects the best platform HTTP stack, supports HTTP/2 and QUIC, and can own an on-disk HTTP cache; Brotli can be enabled on the builder ([`HttpEngine`](https://developer.android.com/reference/android/net/http/HttpEngine), [`HttpEngine.Builder`](https://developer.android.com/reference/android/net/http/HttpEngine.Builder), [`UrlRequest.Callback`](https://developer.android.com/reference/android/net/http/UrlRequest.Callback.html)). This is a deep implementation behind a small Ink interface: callers gain TLS, redirects, compression, caching, deduplication, limits and lifecycle cancellation without learning Android networking.

Decode remote images in Rust through Android's NDK `AImageDecoder`, then upload the result through the renderer's existing wgpu image path. `AImageDecoder` is available from API 30, supports JPEG, PNG, GIF, WebP, BMP, ICO, WBMP, HEIF and DNG, can decode from a buffer or file descriptor directly into caller-owned RGBA memory, and can downscale during decode ([NDK image decoder reference](https://developer.android.com/ndk/reference/group/image-decoder), [NDK image decoder guide](https://developer.android.com/ndk/guides/image-decoder)). This avoids adding JPEG and WebP decoder crates to every networked APK. Animated formats should render their first frame only: Ink has no animation loop and should not acquire one accidentally.

## Proposed public interface

This is the flexible end-state. The first implementation can omit `http.action()` until an application needs a response-bearing write, without weakening the read or image seam.

```ts
// @ink/network
import type { AsyncResource, ResourceError } from "ink";

export type HttpPrimitive = string | number | boolean;
export type HttpQueryValue = HttpPrimitive | null | ReadonlyArray<HttpPrimitive>;
export type HttpHeaders = Readonly<Record<string, string>>;
export type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";

export interface HttpRequest {
  readonly url: string;
  readonly query?: Readonly<Record<string, HttpQueryValue>>;
  readonly headers?: HttpHeaders;
  readonly cache?: "default" | "no-store";
  readonly timeoutMs?: number;
  readonly maxBytes?: number;
}

export interface HttpJsonRequest extends HttpRequest {
  readonly method?: HttpMethod;
  readonly json?: Ink.JsonValue;
}

export type HttpJsonReadRequest = Omit<HttpJsonRequest, "method"> & {
  readonly method?: "GET" | "POST";
};

type HttpErrorBase = Omit<ResourceError, "kind">;

export type HttpError =
  | (HttpErrorBase & { readonly kind: "network" })
  | (HttpErrorBase & {
      readonly kind: "http";
      readonly status: number;
    })
  | (HttpErrorBase & { readonly kind: "invalid-response" })
  | (HttpErrorBase & { readonly kind: "response-too-large" })
  | (HttpErrorBase & { readonly kind: "timeout" });

export interface AsyncAction<T, E = ResourceError> {
  readonly status: "idle" | "running" | "success" | "error";
  readonly value?: T;
  readonly error?: E;
  run(): void;
  reset(): void;
}

export declare const http: {
  json<T extends Ink.JsonValue>(request: HttpJsonReadRequest): AsyncResource<T, HttpError>;
  text(request: HttpRequest): AsyncResource<string, HttpError>;
  action<T extends Ink.JsonValue = true>(
    request: HttpJsonRequest & { readonly method: "POST" | "PUT" | "PATCH" | "DELETE" },
  ): AsyncAction<T, HttpError>;
};
```

`AsyncAction` should be a discriminated union in the real declarations, just like `AsyncResource`, so `value` exists only for `"success"` and `error` only for `"error"`. It is abbreviated above to keep the networking proposal readable.

Extend the existing image source rather than add `RemoteImage`:

```ts
export type ImageSource =
  | string
  | {
      readonly uri: string;
      readonly headers?: HttpHeaders;
      readonly cache?: "default" | "no-store";
    };

interface ImageProps {
  src: ImageSource;
  fallback?: string;
  width: number;
  height: number;
  fit?: "cover" | "contain";
}
```

A plain string remains the common interface: a local path is a compile-time asset and an `https://` value is remote. The object form exists only for authenticated image endpoints or explicit cache control. `fallback` must be a local compile-time image, keeping failure deterministic and avoiding recursive downloads.

### Data model and compile-time schema

`http.json<T>()` is not an unvalidated cast. The compiler resolves `T` and lowers it to a data schema alongside the generated Rust application. The supported model should be deliberately data-only:

- `string`, finite `number`, `boolean` and `null`;
- readonly objects and arrays;
- optional object properties;
- string, number and boolean literal unions.

Functions, classes, index signatures, `unknown`, `any`, recursive aliases and arbitrary conditional/generic types should be compile errors. Unknown response fields can be ignored, but required fields, literal unions and scalar kinds must be checked. This requires extending Ink's current integer-only `StateValue` with finite floating-point numbers, null and optional fields. `serde_json::Value` already represents null, booleans, integer or floating-point numbers, strings, arrays and objects; its deserializer retains a recursion limit unless the explicitly dangerous `unbounded_depth` feature is enabled ([`serde_json::Value`](https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html), [deserializer source](https://docs.rs/serde_json/latest/src/serde_json/de.rs.html#33-68)). Ink should keep that limit and validate the parsed value against its generated schema.

## Usage

### Weather

```tsx
import { http } from "@ink/network";

type Forecast = {
  current: {
    temperature_2m: number;
    weather_code: number;
  };
};

export default function Weather() {
  const unit = persistedState<"celsius" | "fahrenheit">(
    "temperature-unit",
    "celsius",
  );
  const forecast = http.json<Forecast>({
    url: "https://api.open-meteo.com/v1/forecast",
    query: {
      latitude: 51.5072,
      longitude: -0.1276,
      current: "temperature_2m,weather_code",
      temperature_unit: unit.value,
    },
  });

  return (
    <Screen title="Weather">
      {forecast.status === "loading" && <Text>Loading weather</Text>}
      {forecast.status === "error" && (
        <Button onPress={forecast.reload}>Try again</Button>
      )}
      {forecast.status === "ready" && (
        <Text size={64}>{forecast.value.current.temperature_2m}</Text>
      )}
    </Screen>
  );
}
```

Reactive values used in a request are its dependencies. Changing `unit.value` while the screen is active should cancel the old request and start the new request automatically; no effect hook or manual query key is required. A canonical request identity is an implementation detail, not caller configuration.

### Spotify or Beeper imagery

```tsx
<Image
  src={track.artworkUrl}
  fallback="./assets/artwork-fallback.png"
  width={132}
  height={132}
  fit="cover"
/>
```

```tsx
<Image
  src={{
    uri: room.avatarUrl,
    headers: { Authorization: `Bearer ${accessToken.value}` },
    cache: "no-store",
  }}
  fallback="./assets/avatar-fallback.png"
  width={72}
  height={72}
  fit="cover"
/>
```

Remote image loading should be invisible to screen code. Only images in the viewport and a small look-ahead region should activate. Equal active requests share one download and decode; leaving that region releases the consumer and cancels work when the last consumer disappears.

### A future Beeper write

```tsx
const send = http.action<{ id: string }>({
  method: "POST",
  url: "https://api.example.com/messages",
  headers: { Authorization: `Bearer ${accessToken.value}` },
  json: { roomId: room.id, text: draft.value },
  cache: "no-store",
});

<Button onPress={send.run}>Send</Button>
```

The action recipe reads current reactive values when `run()` is pressed. It is never executed merely because a screen became visible. Ink should not automatically retry write actions: idempotency is owned by the remote endpoint, not inferable from an HTTP verb.

## Interface invariants

### Requests

- Release builds accept HTTPS only. Android disables cleartext by default for applications targeting API 28 or newer; Ink should still reject `http://` itself so behaviour does not depend on a client honouring Android's policy ([Android network security configuration](https://developer.android.com/privacy-and-security/security-config), [cleartext risk guidance](https://developer.android.com/privacy-and-security/risks/cleartext-communications)). A narrowly scoped development exception can come later if local-device work requires it.
- The compiler adds `android.permission.INTERNET` and `android.permission.ACCESS_NETWORK_STATE` only when a network resource or remote image exists. Both are install-time normal permissions, not runtime prompts. Android's platform `HttpEngine` reads the active network internally and requires the latter even though Ink does not expose connectivity state ([Android networking guide](https://developer.android.com/develop/connectivity/network-ops/connecting)).
- Request URLs, query values, headers and JSON bodies may contain literals and supported reactive scalar values. Arbitrary JavaScript functions and promise chains are not part of the interface.
- Query encoding is owned by the module. Callers provide values, not pre-escaped fragments. Null omits a key; arrays repeat the key in order.
- Ink owns `Host`, `Content-Length`, `Transfer-Encoding`, `Connection`, `Accept-Encoding` and other hop-by-hop/framing headers. Rejecting them prevents callers from breaking compression, pooling or body accounting.
- Defaults should be finite: one overall deadline, a strict decoded-body limit and a redirect limit of five. `timeoutMs` and `maxBytes` can only narrow or raise defaults within framework hard caps. Both reqwest and ureq otherwise permit surprisingly broad defaults: reqwest's total timeout is absent unless configured and it follows ten redirects, while ureq's convenience body readers default to 10 MB and its redirect default is also ten ([reqwest `ClientBuilder`](https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html), [ureq body limits](https://docs.rs/ureq/latest/ureq/struct.Body.html), [ureq redirect error](https://docs.rs/ureq/latest/ureq/enum.Error.html)).
- A 2xx response is success. Other final statuses become `HttpError` with `kind: "http"` and a status code. Response bodies must not be copied into public error messages because they commonly contain secrets or unbounded server text.
- Redirects remain under Ink's control through `onRedirectReceived`. Follow at most five HTTPS redirects. Strip credentials when origin changes; do not automatically redirect a non-idempotent action across origins. The HTTP specification scopes authentication credentials to an origin's protection space ([RFC 9110, authentication](https://www.rfc-editor.org/rfc/rfc9110.html#section-11.5)).

### Resources and actions

- Reads remain lazy and screen-owned. One resource has at most one request in flight; changing a dependency or calling `reload()` cancels the old request and stale completions cannot win.
- Identical active reads are single-flight across screens. The identity includes method, canonical URL/query, relevant headers, body, response kind and schema. Sensitive identity material is hashed and never logged.
- `reload()` asks the HTTP cache to revalidate rather than deleting the cached entry. `cache: "no-store"` bypasses storage. Ink should follow origin cache directives instead of inventing a separate duration vocabulary: RFC 9111 defines freshness, validation and directives such as `no-store`, `no-cache`, `max-age` and `must-revalidate` ([RFC 9111](https://www.rfc-editor.org/rfc/rfc9111.html)).
- Actions are cold until `run()`, single-flight by default, cancellable with their owning screen, and never automatically retried. A later explicit concurrency option should be justified by a real application.

### Remote images

- Width and height remain required. They determine layout immediately and give the decoder a physical target size before allocating pixels.
- Enforce a compressed-byte cap, source-dimension cap, decoded-pixel cap and the wgpu device's `max_texture_dimension_2d`. Android notes that RGBA8888 costs four bytes per pixel and that full-resolution images are a common source of memory pressure ([Android bitmap memory](https://developer.android.com/topic/performance/memory/guide/bitmaps)); wgpu exposes the actual 2D texture dimension limit and recommends requesting only the limits an application needs ([wgpu `Limits`](https://docs.rs/wgpu/latest/wgpu/struct.Limits.html)).
- Decode near the physical draw extent, preserving enough source pixels for `cover` or `contain`, and never upscale during decode. Android recommends target-sized decoding to avoid memory with no visible benefit, and `AImageDecoder_setTargetSize` explicitly recommends downscaling only ([large bitmap guidance](https://developer.android.com/topic/performance/graphics/load-bitmap), [`AImageDecoder_setTargetSize`](https://developer.android.com/ndk/reference/group/image-decoder#aimagedecoder_settargetsize)).
- Convert output to sRGB RGBA8888. `AImageDecoder_setDataSpace` can transform to a requested data space, while `ADATASPACE_SRGB` specifies full-range sRGB/BT.709 ([NDK image decoder data space](https://developer.android.com/ndk/reference/group/image-decoder#aimagedecoder_setdataspace), [NDK data spaces](https://developer.android.com/ndk/reference/group/a-data-space)).
- Use one alpha convention end to end. `AImageDecoder` premultiplies alpha by default; wgpu supplies a matching `PREMULTIPLIED_ALPHA_BLENDING` state ([NDK image alpha behaviour](https://developer.android.com/ndk/reference/group/image-decoder#aimagedecoderheaderinfo_getalphaflags), [wgpu `BlendState`](https://docs.rs/wgpu/latest/wgpu/struct.BlendState.html)). Local image pixels should be premultiplied too if the renderer adopts that path.
- Upload tightly packed rows with `Queue::write_texture`. Unlike buffer-to-texture copies, this method does not require 256-byte row alignment, so Ink need not allocate a padded copy ([wgpu `TexelCopyBufferLayout`](https://docs.rs/wgpu/latest/wgpu/struct.TexelCopyBufferLayout.html)). The renderer should upload on its owning thread, then release the CPU RGBA buffer after submission.
- Cache compressed HTTP responses on disk and decoded GPU textures in a bounded, byte-accounted LRU. The key includes the response cache identity, target physical dimensions and fit. Eviction, device loss and app restart are hidden implementation details.

## Errors

The public error interface should stay stable even when Android's underlying errors change:

| Kind | Meaning | Retryable default |
| --- | --- | --- |
| `network` | DNS, connection, TLS or transport failure before a valid final response | `true`, except certificate/policy failures |
| `http` | Final non-2xx response; includes `status` | `true` for 408, 429 and 5xx; otherwise `false` |
| `invalid-response` | Invalid UTF-8/JSON, schema mismatch, unsupported image or decode failure | `false` |
| `response-too-large` | Encoded body, decoded pixels or dimensions exceeded a hard limit | `false` |
| `timeout` | Framework deadline elapsed | `true` for reads, not automatically retried |

Cancellation is lifecycle, not an application error: a resource leaving scope returns to its cached/inactive state, and an action explicitly reset or left behind returns to idle. This avoids UI branches for an event the framework itself initiated.

Image failures normally render the local fallback and do not expose an error callback. If a future application genuinely needs image error logic, an explicit `imageResource()` can be added then; adding load events now would make the image module shallower by leaking its lifecycle through every call site.

## Hidden implementation

### Compiler and core

1. Recognise `@ink/network` constructors, resolve the generic response type through Oxc, reject unsupported type shapes and lower a response schema into Ink IR.
2. Lower request fields to recipes rather than static payload strings. Each recipe records literals plus references to signals, current list items or resource fields. The engine evaluates it when activating/reloading/running and tracks those references as dependencies.
3. Extend `StateShape`/`StateValue` for floating-point numbers, null and optional object fields. Parse JSON in Rust with `serde_json`, then validate and project it through the generated schema. `serde_json::from_slice` reports syntax and type/data mismatches; Ink adds a short JSON path to the structured error ([`serde_json::from_slice`](https://docs.rs/serde_json/latest/serde_json/fn.from_slice.html), [`serde_json::Error`](https://docs.rs/serde_json/latest/serde_json/error/struct.Error.html)).
4. Add binary/file native results rather than Base64. JSON/text and image bodies must not expand by one third or create multiple giant strings merely to cross JNI.
5. Represent remote images as internal image resources owned by visible display-list entries. Their download/decode completion invalidates the scene just like any other resource.

### Android networking adapter

Create one process-wide `HttpEngine` only when generated capabilities include networking. Configure its private disk directory, bounded cache and Brotli once. Android recommends a single engine and prohibits concurrent engines sharing a storage directory; its builder's disk-cache size is advisory ([`HttpEngine.Builder`](https://developer.android.com/reference/android/net/http/HttpEngine.Builder)).

Each native request owns a `UrlRequest`, redirect counter, strict output counter and temporary file. Callbacks run on a dedicated executor, write response chunks to the temporary file, and atomically publish the file only on success. `UrlRequest.cancel()` gives the existing Ink cancellation request a direct implementation, after which no callback other than `onCanceled` is delivered ([`UrlRequest.Callback.onCanceled`](https://developer.android.com/reference/android/net/http/UrlRequest.Callback.html#onCanceled(android.net.http.UrlRequest,android.net.http.UrlResponseInfo))). Rust receives a compact result descriptor with status, selected headers and an app-private file path/descriptor; it never receives Base64.

`HttpEngine`'s received-byte metric is measured before decompression, so it is useful for diagnostics but cannot enforce the decoded body limit. The adapter must count bytes actually delivered through `onReadCompleted` ([`UrlResponseInfo.getReceivedByteCount`](https://developer.android.com/reference/android/net/http/UrlResponseInfo#getReceivedByteCount())).

### Image decoder and renderer

Open the cached encoded body through `AImageDecoder_createFromFd` or a mapped buffer, inspect width/height/MIME before allocation, choose an efficient sampled target, request RGBA8888 and sRGB, then decode directly into a Rust-owned vector. The decoder documentation defines the minimum stride and required buffer size, so allocation can be checked before decode ([`AImageDecoder_getMinimumStride`](https://developer.android.com/ndk/reference/group/image-decoder#aimagedecoder_getminimumstride), [`AImageDecoder_decodeImage`](https://developer.android.com/ndk/reference/group/image-decoder#aimagedecoder_decodeimage)).

Pass the decoded allocation to `ink-renderer-wgpu`, create an `Rgba8UnormSrgb` texture and call `Queue::write_texture`. Replace the current unbounded image `HashMap` with byte-accounted LRU ownership shared by local and remote images. The cache owns textures and bind groups; scenes hold cheap image keys.

## Dependency categories and adapters

- Request canonicalisation, schema validation, cache identity and error mapping are **in-process** dependencies. They belong inside the deep network module and are tested through its interface; no adapter is warranted.
- Android `HttpEngine` and `AImageDecoder` are **local-substitutable** platform dependencies in the emulator/device environment. They sit behind Ink's existing native request and renderer seams. `HttpEngineAdapter` and `AImageDecoderAdapter` are production adapters, not new app-facing interfaces.
- Weather, Spotify, Beeper and arbitrary application endpoints are **true external** dependencies. Ink owns transport correctness but cannot own their availability, schema evolution or authentication policy. Application packages define their domain types and thin endpoint functions on top of `http.json`; the framework should not contain Weather- or Spotify-specific methods.
- The existing native request dispatcher is already a real seam because Light SDK and networking are distinct adapters. Do not add another public `NetworkClient` port merely for tests. Tests and local tooling can replace the adapter at that internal seam without reducing locality in application code.

This seam placement concentrates platform changes in Android, schema/lifecycle changes in core, and drawing changes in the renderer. Deleting the module would force TLS policy, request identity, cancellation, response validation, image limits and caching back into every application, so it earns its depth and gives high leverage.

## Alternatives considered

### reqwest with rustls

reqwest is a capable asynchronous client, with configurable total/read/connect timeouts, redirect policy and optional gzip, Brotli, zstd and deflate decoding. Its TLS backend and compression support are feature-selected, and connect-timeout operation requires a Tokio runtime with timers ([reqwest features](https://docs.rs/reqwest/latest/reqwest/), [`ClientBuilder`](https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html)). It would keep more implementation in Rust but would also add an async runtime plus HTTP, TLS, certificate roots and cryptography to Ink's native library. That works against the Android-only framework's APK and maintenance goals while duplicating a capable platform stack.

### ureq with rustls

ureq is synchronous and easier to drive from a worker pool. Its defaults enable rustls and gzip; platform certificate verification is a separate optional feature. It has detailed phase-specific timeouts and bounded convenience readers ([ureq features](https://docs.rs/ureq/latest/ureq/), [ureq timeouts](https://docs.rs/ureq/latest/ureq/config/struct.Timeouts.html)). It still brings a Rust TLS/crypto stack and blocking worker ownership, while platform `HttpEngine` already gives Ink asynchronous cancellation, caching and modern protocols without an APK dependency.

### `HttpURLConnection`

This is the conservative platform fallback. Android documents built-in TLS, streaming, configurable timeouts, IPv6, pooling, transparent gzip and up to five redirects; the platform `HttpResponseCache` can add a bounded on-disk cache ([Android networking guide](https://developer.android.com/develop/connectivity/network-ops/connecting), [`HttpURLConnection`](https://developer.android.com/reference/java/net/HttpURLConnection), [`HttpResponseCache`](https://developer.android.com/reference/android/net/http/HttpResponseCache)). It is blocking and its cancellation/stream lifecycle is less direct than `UrlRequest.cancel()`. Do not maintain both adapters initially: one adapter means a hypothetical choice. Use `HttpEngine` on the LP3 and add a fallback only if device verification exposes a real incompatibility.

### Rust `image` crate or Android `BitmapFactory`

The Rust `image` crate can explicitly enable only PNG, JPEG and WebP rather than its broad default format set, and offers width, height and allocation limits; only dimension limits are currently strict ([image feature list](https://docs.rs/crate/image/latest), [`image::Limits`](https://docs.rs/image/latest/image/struct.Limits.html)). It is a reasonable cross-platform adapter, but Ink is Android-only and can avoid decoder code in the APK through `AImageDecoder`.

`BitmapFactory` also uses platform codecs and supports bounds-only/downsampled decode, but creates a Java `Bitmap` before Ink can copy RGBA pixels into its Rust/wgpu texture ([`BitmapFactory`](https://developer.android.com/reference/android/graphics/BitmapFactory), [`BitmapFactory.Options`](https://developer.android.com/reference/android/graphics/BitmapFactory.Options)). `AImageDecoder` writes into caller-owned native memory and is therefore the cleaner seam.

## Trade-offs

The interface is flexible enough for arbitrary authenticated APIs, query-dependent resources, JSON reads, text reads and eventually response-bearing writes. That flexibility costs compiler work: Ink must resolve data-only TypeScript types, lower reactive request recipes and expand its value model. This is still preferable to compiling arbitrary async TypeScript, which would turn JavaScript execution semantics into the framework interface and erase the ahead-of-time advantage.

Direct URL images provide excellent common-case DX and high leverage, but deliberately hide load progress and failure. The fixed-size/fallback invariant is a good fit for Ink's text-and-image applications; an app that wants a bespoke progressive image state would need a later explicit resource.

`HttpEngine` maximises APK and maintenance locality but ties the networking implementation to Android API 34. That is not a new product constraint for Ink. Its callback adapter is more code than blocking `HttpURLConnection`, but cancellation, HTTP/2/3, compression and cache ownership stay in the platform module rather than being rebuilt across Rust crates.

The strongest incremental implementation is:

1. `http.json<T>()` GET resources with reactive query/headers, strict schemas, limits, cancellation and conditional `INTERNET`.
2. Direct HTTPS `Image.src`, viewport activation, platform decoding and bounded texture/cache ownership.
3. `http.text()` when a real endpoint needs it.
4. Observable `http.action()` with a real Beeper/Spotify write flow, rather than inventing action concurrency and retry semantics speculatively.

This sequence keeps the external interface coherent while allowing the implementation to deepen behind the same seams.
