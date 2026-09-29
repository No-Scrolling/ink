# Reverb file-loading profile

29 September 2026. Light Phone III emulator, Android arm64 debug build, Media3 1.10.1. Three fresh-process launches, each followed by two further song selections: nine samples across the first three MP3s in the emulator's existing Continuum album. No host builds ran during sampling. These are process-cold, **not disk-cache-cold**, measurements; they do not establish LP3 storage or speaker latency.

## Findings

File opening and parsing are relatively small costs. Most of the warm-start interval is audio preparation/output, including a delay after the player is ready. A fresh process additionally creates and connects the playback service.

Sequential stages, mean milliseconds (columns sum to their totals before rounding):

| Stage | Fresh process (3) | Subsequent selection (6) |
| --- | ---: | ---: |
| JS tap handler → native player connected | 66.3 | 3.7 |
| Queue construction and submission | 3.3 | 4.3 |
| Queue submission → file open begins | 9.3 | 8.0 |
| File open call | 6.5 | 3.8 |
| Open returns → MP3 identification completes | 8.2 | 2.5 |
| Identification → AudioTrack initialised | 58.7 | 43.8 |
| AudioTrack initialised → player ready | 5.7 | 0.8 |
| Player ready → estimated audio playout starts | 88.0 | 86.2 |
| **Total** | **246.0** | **153.2** |

The identification-to-output interval includes overlapping extraction, decoder configuration, thread scheduling and AudioTrack setup. It is **not** all file loading.

Additional timings overlap the stages above and must not be added to them:

| Measured operation | Fresh process | Subsequent selection |
| --- | ---: | ---: |
| MP3 sniff/metadata identification call | 2.1 ms | 1.8 ms |
| Cumulative DataSource read calls through roughly 1.18 MB | 12.6 ms | 11.2 ms |
| First 2,048 extractor read calls, including their file reads (~1.13 MB) | 25.2 ms | 17.5 ms |
| Codec initialisation | 19.0 ms | Decoder reused; no new initialisation event |

These files involve approximately 2,150 small DataSource reads for the first 1.18 MB. Cumulative read time includes OS/provider/cache behaviour, not just physical storage. Read-ahead overlaps playback preparation and may extend beyond the point at which enough audio is available to start.

## Experiment

Reduced the MP3 progressive source's loading-check interval from the default to 64 KiB, retaining all buffering thresholds. Repeated the same nine selections with identical instrumentation:

| Mean tap-handler → estimated audio start | Baseline | 64 KiB loading checks |
| --- | ---: | ---: |
| Fresh process | 246.0 ms | 258.3 ms |
| Subsequent selection | 153.2 ms | 155.2 ms |

No improvement demonstrated. Reverted the setting. Earlier exploratory queue reuse and a reduced start-buffer threshold also lacked convincing evidence of an end-to-end gain; neither is retained. An app-level subscription experiment was not measured and was also removed.

No production changes remain from this investigation. Reverb's original source was restored byte-for-byte, type-checked, rebuilt and reinstalled over the existing emulator app without clearing its data. The physical LP3 was not changed.

## Measurement boundary

Start: `Date.now()` inside the existing JS song-tap handler. End: Media3's `onAudioPositionAdvancing` **playoutStartSystemTimeMs**, not the later callback delivery time. This estimates Android audio playout; it does not include touch sensing, emulator host audio output or an acoustic measurement. Native/log timestamps use the same Android wall clock. Instrumentation adds overhead; small differences are inconclusive.

File open and read calls, MP3 sniffing and extractor reads use `System.nanoTime()` elapsed durations. Existing codec/output diagnostics provide the other markers. Each sample starts from the album screen and selects a different one of the first three tracks. Audio reported advancing in every accepted sample. No playback errors or underruns were found in the accepted measurement logs.

Evidence: [baseline stages](extractor-profile.json), [baseline markers](extractor-profile.log), [candidate stages](loading-64k.json), [candidate markers](loading-64k.log). [Profiling patch](profiling.patch) contains temporary instrumentation against the restored Reverb files; it is not applied to production. Full logcat recordings remain in the ignored `.agent-tools/reverb-playing/` directory.

## Next targets

1. Profile AudioTrack startup/output on the physical LP3. The emulator's ~86 ms after READY is the largest warm-start interval, but its audio backend differs from the phone.
2. For first selection, evaluate connecting/preparing the service while browsing, including its startup and idle resource costs. This moves work earlier rather than reducing file I/O.
3. If LP3 file timings are materially higher, test bounded buffered reads and large embedded-artwork cases. Preserve accurate seeking, gapless metadata and playback reliability; these MP3-only samples do not justify changing other formats.

Media3 references: [audio position callback](https://github.com/androidx/media/blob/1.10.1/libraries/exoplayer/src/main/java/androidx/media3/exoplayer/audio/AudioRendererEventListener.java), [progressive loading checks](https://github.com/androidx/media/blob/1.10.1/libraries/exoplayer/src/main/java/androidx/media3/exoplayer/source/ProgressiveMediaPeriod.java), [buffer defaults](https://github.com/androidx/media/blob/1.10.1/libraries/exoplayer/src/main/java/androidx/media3/exoplayer/DefaultLoadControl.java).
