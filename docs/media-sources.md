---
title: "Native media sources"
description: "Connect app-specific audio protocols to Ink playback and downloads."
---

Native media sources let an Android extension resolve a stable media address and supply its bytes to Ink’s player or download scheduler. Use them for protocols that need native authentication, decryption or source resolution.

This API requires the development SDK. It is not included in v0.1.0-alpha.10.

## Register a source

Add your extension under `android/kotlin/` and register its factory in `android/src/main/AndroidManifest.xml`:

```xml
<application>
  <meta-data android:name="ink.mediaSourceFactory" android:value="com.example.music.MediaSources" />
</application>
```

The factory implements `com.vandam.ink.InkMediaSourceFactory`. Its `create(context)` method returns a map from provider names to `InkMediaSource` instances. Names accept a lowercase letter followed by up to 63 lowercase letters, digits or hyphens.

For a provider named `catalogue`, addresses start with `ink-source://catalogue/`. The provider defines the remaining path. Reject malformed or unsupported addresses in the provider.

Factories have a public no-argument constructor. Preserve it in the extension’s consumer ProGuard rules. Ink creates the factory once per process using the application context. It can be created by a background download before the app’s activity opens. Do not retain an activity or depend on JavaScript initialisation.

## Supply audio bytes

Implement `open(uri, position, identity, purpose)`. Ink calls it on a worker thread. Return an `InkMediaStream` with an input stream, its starting byte position, the full resource length or `null`, a non-empty content identity and a MIME type.

Playback and downloads call the cancellation-aware overload, `open(uri, position, identity, purpose, cancelled)`. Its default implementation delegates to the four-argument method. Override it when opening a source can wait on a connection or retry: check the `BooleanSupplier` with `cancelled.asBoolean` and stop the open operation when it returns `true`. For playback, the supplier becomes true when Ink releases the track. Cancellation must release any pending network requests before a stream exists to close.

For `InkMediaPurpose.PLAYBACK`, return playable, decrypted bytes at exactly the requested position. Positions and lengths refer to that representation, after removing any protocol headers. Keep resource identities and byte positions stable across opens.

If the stream supports seeking, pass a `seek` callback as the last `InkMediaStream` argument. The callback moves the input to an absolute byte position in the playable resource and discards any buffered bytes from its previous position. Ink retains the stream across decoder seeks and closes it when releasing the track. Without this callback, Ink reopens the source for each seek. Downloads always close their streams after each transfer.

For a temporary playback failure, throw `InkMediaRetryException(message, retryAfterMillis, maxRetries)` from `open`. Ink retries the load after that delay, up to the specified limit. The default limit is `3`; `0` fails immediately. Other playback I/O failures use Media3’s default retry delays and limits. A custom load error policy supplied to the media source factory overrides this behaviour.

For `InkMediaPurpose.DOWNLOAD`, return the bytes to retain on disc. These may remain encrypted. If the requested content identity matches the current resource, resume at the requested position. Otherwise return position `0` and the new identity; Ink truncates the partial file and restarts.

Closing the stream must interrupt pending reads and release native resources. An open or read failure fails the current operation. The provider handles authentication and URL refresh whenever Ink opens it.

## Download and play protected media

Enqueue the stable source address through `@ink/network/downloads`:

```ts
import { downloads } from "@ink/network/downloads";

const job = await downloads.enqueue({
  key: track.id,
  url: `ink-source://catalogue/tracks/${track.id}`,
  name: track.title,
});
```

This example requires a registered `catalogue` source and an app-defined `track`.

Ink schedules and persists the transfer, tracks progress and validates resumed byte ranges. Before publishing completion, it calls `downloaded(uri, id, file)` with the private managed file. Record the file in your provider’s durable index so playback can reopen and decrypt it without a network request. Throw if the index cannot be saved; Ink removes the incomplete file and reports failure.

For a temporary provider failure, throw `InkMediaRetryException(message, retryAfterMillis, maxRetries)` from `open`. Use a positive delay in milliseconds. `maxRetries` defaults to `3`; set it to `0` to fail immediately. Ink returns the download to `queued` and schedules another attempt after that delay, keeping any partial file. The worker is released while waiting, and the user can pause or cancel the queued download. After the retry limit, the download fails with the provider’s message. Provider and I/O retries share the download’s attempt counter, which resets on manual resume.

Ink also retries `IOException` failures up to three times, after delays of 1, 3 and 10 seconds. Other exceptions mark the download as failed. Manually resuming a failed download resets this retry limit.

Keep using the stable provider address for playback. The completed job’s `file.src` refers to the stored download bytes, which may be encrypted and unsuitable for direct playback. Keep credentials and decryption keys in native secure storage.

Ink calls `removed(uri, id)` when removing a provider download. Remove the corresponding index entry; the callback must also tolerate a partially committed download. Ink removes its managed file separately. Pausing or cancelling an unfinished transfer does not call this callback.

HTTP(S) downloads retain their existing URL, redirect and network policy checks. Custom sources are app-owned native code and enforce their own connection policies.
