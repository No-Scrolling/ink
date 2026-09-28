# Native GIF playback

Beeper now uses the original GIF for conversation thumbnails instead of a still preview. Attachment MIME types are retained in its cache, and GIF detection also recognises filenames/URLs and the GIF attachment type. The media viewer recognises those attachments as images.

Ink's Android image path now advances frames with the existing system decoder. It keeps a compositing buffer rather than decoding every frame up front, preserves transparency/disposal, and reuses the GPU texture between frames. Static images retain their existing decoding path. GIF imports are supported alongside other bundled images; remote and bundled images use the same file decoder.

`Message` and `ConversationScreen` images always loop, including files with a finite repeat count. Standalone `Image` respects the file's repeat count unless `loop` is supplied. Only visible images advance; hidden images retain their current frame. Decoder buffers are included conservatively in the image-cache budget and released with evicted images. Frame delays are clamped to 20 ms; missing/non-positive delays use 100 ms.

The compositing buffer remains premultiplied for Android's frame blending; a separate frame is converted to straight alpha for Ink. Android owns disposal and partial-frame composition. See [Android Image Decoder](https://developer.android.com/ndk/reference/group/image-decoder).

## Validation

- Beeper development APK built successfully; Beeper and Ink TypeScript checks passed.
- Existing core/runtime tests: 8 passed. Core also checks without default features. No new unit tests were added.
- An isolated emulator fixture displayed three-frame transparent GIFs, with 400/600/800 ms frame durations. Both thumbnails and an enlarged zoomable image showed all three colours. Pixel comparisons confirmed movement within the image area and no previous-colour trails: [thumbnail red](thumbnail-red.png), [thumbnail blue](thumbnail-blue.png), [viewer red](viewer-red.png), [viewer blue](viewer-blue.png).
- Without a looping override, the finite GIF settled on its last frame while the neighbouring infinite GIF continued changing ([samples](finite-frames.json)).
- Hiding images produced zero process CPU ticks over a two-second sample ([record](hidden-cpu.json)); this is an idle sanity check, not a performance benchmark.
- The final fixture used the finite GIF inside `ConversationScreen`. After waiting five seconds (longer than its 3.6-second encoded playback), all three colours continued appearing across another seven seconds: [samples](conversation-frames.json), [green frame](conversation-green.png), [blue frame](conversation-blue.png). The filtered final runtime error log was empty.

Initial playback experiment: `experiment-20260929-003045-ecfae57b`. Final conversation-loop experiment: `experiment-20260929-003721-acf02854`; [manifest](manifest.json). The fixture overlays the Counter app only in an isolated source snapshot, with temporary package `com.vandam.benchmark.ink.gif`. Its source and small GIF assets are retained here. The emulator closed between checks and was relaunched visibly before the final run.

The temporary app was removed, the reservation released and build workspaces cleaned. Beeper's actual authenticated conversations and LP3 GIF performance were not exercised; the Beeper APK build and shared native image/conversation paths were validated separately. Rebuild Beeper with `ink dev` to use the changes.
