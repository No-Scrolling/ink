# Rust / AAudio feasibility on LP3

29 September 2026. Physical TLP301 / Android 14, built-in speaker (audio-policy port 3), 48 kHz stereo float PCM. Rust control/render code and direct AAudio FFI; no React or Media3. The final real-song runs used shared mode with low-latency mode granted, 96-frame bursts and a 192-frame buffer. No underruns were reported.

## Main result

**The isolated warm-stream file-open → decode → estimated output path stayed below 40 ms in all 16 trials across three local MP3s.** This demonstrates a feasible component path towards sub-100 ms playback, not that Reverb now meets that target.

| Shared output, Man Overboard | Samples | Mean | Range |
| --- | ---: | ---: | ---: |
| Prepared PCM; create/open/start stream on demand | 5 | 233.7 ms | 212.3–251.6 ms |
| Prepared PCM; stream already open but stopped | 5 | 107.0 ms | 97.4–117.9 ms |
| Prepared PCM; stream running silence | 15 | **13.7 ms** | 13.1–14.7 ms |
| Open MP3, decode/resample prefix, publish to running stream | 10 | **36.6 ms** | 34.5–39.0 ms |

The last row includes reopening the file and creating a new decoder for each request. Within that interval, file/decode/resampling averaged 22.5 ms, callback arrival averaged 23.5 ms after the request, and endpoint output averaged 36.6 ms. These are nested timings, not additional costs.

Quick repeat checks with two different songs:

| MP3 | Samples | Mean open/decode/output | Maximum |
| --- | ---: | ---: | ---: |
| No Feeling | 3 | 36.7 ms | 37.1 ms |
| Blood on My Blood | 3 | 36.4 ms | 37.3 ms |

The independent five-decode preparation loop before stream setup averaged 3.0 ms for Man Overboard. This ran in a different scheduling/workload context from requests spaced out while the stream was active. The combined 36.6 ms result is the relevant on-demand measurement; adding the separate 3.0 ms figure to output latency would be misleading.

## Keeping the stream warm has a cost

During a two-second silent window, this process used 54.87 ms CPU and received 1,000 callbacks: about **2.7% of one CPU core and 500 callbacks/second**. This is a short client-process measurement. It excludes AAudio/AudioFlinger service CPU, DSP/amplifier power and battery drain.

An earlier quiet-tone comparison measured warm shared output at 14.4 ms and exclusive output at 8.1 ms, both with no underruns. Exclusive access is unnecessary for the demonstrated sub-100 ms path and is less suitable for coexisting with other audio. The final song tests use shared mode.

Simply opening a stream before the tap did not consistently reach 100 ms. It needs to be started and feeding silence for the demonstrated low-latency path. The first song can use that path only if the stream has been warmed before the request. A genuinely cold stream still took over 200 ms here.

## What was built

[Standalone benchmark](../../audio-startup/README.md): Rust callback, predecoded clip triggering, bounded on-demand MP3-prefix decoding with Symphonia 0.5.5, and AAudio endpoint timestamp collection. Its dependencies are isolated from Ink's workspace and Reverb. A quick run takes a few seconds after compilation.

The real-file path decodes 200 ms of stereo MP3, performs a prototype-only linear conversion to 48 kHz, then publishes the clip to the callback. File reading, allocations and decoding stay outside the real-time callback. Clip backing allocations stay alive until stream close; the fixture does not implement a production ring buffer or continuous decoder.

This supports moving the audio data path into Rust, but it does not justify replacing the production player wholesale yet. The next app-level experiment should preserve Android media sessions, audio focus and background playback, use proper URI/FD access and quality resampling, and measure a real Reverb tap through to the same endpoint timestamps. It also needs full-song, seek, pause/resume, gapless, format, route-change and idle-power checks, plus a bounded warm-stream policy.

## Measurement limits

- Start is a native benchmark request, not a physical tap or React event. File mode starts before file opening; prepared mode starts after PCM exists in memory.
- End is the first supplied PCM frame's estimated endpoint presentation time. It is computed from the median of five advancing AAudio timestamps, with raw frame/timestamp pairs and spread retained. It is not microphone-measured sound, and intrinsic silence at the beginning of a song is not included in the latency claim.
- The process runs as Android shell with direct file paths. Production content-provider/permission handling, JavaScript dispatch, media-session commands and audio focus are excluded.
- Filesystem caches were not flushed. Three MP3s, one speaker route and short clips do not establish all-format/full-app compatibility or long-run reliability.
- Android granted the requested low-latency shared mode. The same configuration is not guaranteed on Bluetooth or other devices.

## Evidence and cleanup

- [Full real-song run](man-overboard.jsonl), [source/binary manifest](man-overboard-manifest.json).
- [Second song](no-feeling.jsonl), [third song](blood-on-my-blood.jsonl), with corresponding manifests.
- [Summary](summary.json), exploratory [shared tone](tone-shared.jsonl) and [exclusive tone](tone-exclusive.jsonl) samples.

The final real-song run has 35 output measurements plus decode and idle records; the two quick runs add 12 output measurements. Every output measurement reported zero xruns. All run commands completed successfully. Source formatting and Python syntax checks passed. The runner removed its remote executables, the initial manually deployed probe was also removed, and the device reservation was released. Reverb's installed app and production audio engine were not replaced in this experiment; no song files were modified or copied to the host.

References: [AAudio overview and lifecycle](https://developer.android.com/ndk/guides/audio/aaudio/aaudio), [Android low-latency audio guidance](https://developer.android.com/games/sdk/oboe/low-latency-audio), [Symphonia](https://github.com/pdeljanov/Symphonia).
