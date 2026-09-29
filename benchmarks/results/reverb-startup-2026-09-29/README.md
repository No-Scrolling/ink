# Reverb first-song startup investigation

29 September 2026. Physical LP3, Android 14, built-in speaker, MP3 / PCM fallback. Same three American Football songs and timing boundary as the [LP3 loading profile](../reverb-lp3-loading-2026-09-29/README.md). Each variant has three fresh-process first selections and six subsequent selections. Instrumented debug builds; sequential variant order, no host builds during measurement. Treat small differences as inconclusive.

## Result

| Mean tap-handler → estimated audio start | Baseline | Connect while browsing | Connect + reduced ID3 parsing |
| --- | ---: | ---: | ---: |
| First song after launching | 707 ms | **448 ms** | 429 ms |
| Subsequent selections | 319 ms | 298 ms | 324 ms |

The retained change removes **259 ms (37%)** from the first song's tap path. Baseline first-song samples ranged from 684–719 ms; preconnected samples ranged from 422–466 ms. It moves service startup earlier rather than making the work disappear. Under 100 ms has **not** been achieved or demonstrated.

Only preconnection is retained. The ID3 variant gave mixed results (19 ms faster first song, 26 ms slower subsequent selections relative to preconnection), insufficient to justify changing metadata behaviour. Its experimental flag and all instrumentation were removed.

## Where first-song connection time goes

Mean breakdown of baseline tap → connected (284.7 ms total):

| Stage | Mean |
| --- | ---: |
| Tap, bridge dispatch and controller construction/bind request | 63.7 ms |
| Bind request → service created | 8.7 ms |
| Service setup before player construction | 33.3 ms |
| Construct/configure ExoPlayer | 65.0 ms |
| Restore saved queue | 29.0 ms |
| Register listeners and create media session | 56.3 ms |
| Session built → controller callback/action | 28.7 ms |

These are wall-clock intervals, including scheduling and class initialisation; they are not isolated CPU-function costs. With preconnection, the tap → connected interval fell to 10.3 ms. The service completed its connection before the first tap in all three measured preconnection rounds. No audio-position-advancing event occurred before that tap.

## Retained implementation

- `components/Tracks.tsx`: connect once when a song list mounts; a failed speculative connection is logged, while an actual play request retains its existing error handling/retry path.
- `lib/player.ts`: small `connectPlayback()` helper.
- `android/kotlin/com/vandam/ink/AppNativeAdapters.kt`: `connect` operation reuses the existing lazy connection. It does not queue, prepare or play a song.

There is no global startup player provider, new timer, decoder preloading, reduced buffer threshold or duplicate player. The existing service/controller is created earlier and retained using its existing lifecycle. This incurs the service's resource cost while browsing, before the first play. A tap immediately after opening a song list can still arrive before connection completes; the measured album dwell was one second plus command overhead. Existing controller futures coalesce the overlapping connection and play requests.

## Remaining first-song waterfall with preconnection

| Stage | Mean |
| --- | ---: |
| Tap → connected | 10.3 ms |
| Queue construction/submission | 18.7 ms |
| Dispatch → file open begins | 53.0 ms |
| Open file | 14.9 ms |
| Open complete → MP3 identified | 36.5 ms |
| MP3 identified → output initialised | 189.7 ms |
| Output initialised → READY | 39.0 ms |
| READY → estimated playout starts | 86.0 ms |
| **Total** | **448.0 ms** |

First codec initialisation within the preparation interval averaged 62.7 ms. Audio reading/extraction, decoder setup and scheduling overlap, so this must not be added to the waterfall total.

## Rust ownership

The measured connection cost is in Android/Media3 service, player and session setup. Rewriting the small JavaScript/Kotlin command wrapper in Rust would still require those operations if it retained the same pipeline. JavaScript is already only selecting the queue and issuing commands.

Rust could own file reading, format parsing/decoding, queue state and a PCM ring buffer, feeding Android's C AAudio API directly. Android integration would still need media-session controls, audio focus, foreground-service lifecycle and route handling. That would be an audio-backend change with format, seeking, gapless, background-playback and power implications, rather than a small bridge optimisation.

The LP3 advertises `android.hardware.audio.low_latency` and `android.hardware.audio.pro`. These flags support investigating a low-latency path; they do not demonstrate sub-100 ms tap-to-sound performance. A useful next isolated prototype would compare a prepared PCM buffer through AAudio against the current sink, measuring cold stream creation separately from a warm stream. Then add real-file decoding and seek/format checks. Keeping audio prepared or an output stream warm trades memory/power for latency.

[Android's low-latency audio guidance](https://developer.android.com/games/sdk/oboe/low-latency-audio) and [Media3 preloading](https://developer.android.com/media/media3/exoplayer/preloading-media/preloadmanager) describe these distinct strategies. Neither establishes this app's achievable latency without measurement.

## Validation and evidence

All 27 accepted selections reported audio advancing, with no playback errors, audio sink errors or underruns in their logs. Thermal status remained 0 at the end of sampling. Reverb type-check and Android build passed after removing instrumentation. The final uninstrumented build was installed on the LP3 without clearing app data. Source changes are limited to the three preconnection files above.

Timing ends at Media3's estimated playout-start timestamp, not callback delivery. It is not a microphone measurement and excludes touch sensing. Tests used cached local files, one output route and a small MP3 sample; Bluetooth, other formats, gapless transitions and battery impact were not comprehensively validated.

Evidence: [baseline](baseline.json), [preconnection](preconnect.json), [metadata experiment](preconnect-metadata.json), corresponding `.log` files, [startup segments](startup-segments.json), and [retained patch](change.patch). Full logs and the temporary instrumentation patch are in the ignored `.agent-tools/reverb-startup/` directory.
