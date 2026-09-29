# Rust / AAudio startup probe

A standalone ARM64 Android executable to separate stream setup, trigger-to-output latency and short MP3 decoding from Reverb's UI and Media3 service. This is experimental benchmarking code, not a replacement music player.

Run against an existing agent-tools reservation (pause other audio first):

```sh
scripts/agent-tools device acquire --serial DEVICE --owner audio-startup
uv run python benchmarks/audio-startup/run.py \
  --serial DEVICE --token TOKEN --ndk /absolute/path/to/android/ndk \
  --mp3 '/existing/path/on/device/song.mp3'
scripts/agent-tools device release --serial DEVICE --token TOKEN
```

The runner validates the reservation, builds the isolated Cargo workspace with the NDK linker, records source/binary hashes, runs it and removes its remote executable. It currently uses the macOS NDK toolchain layout and requires the `aarch64-linux-android` Rust target. Results go to ignored `.agent-tools/aaudio-ID/` directories. No songs are copied to the host. The probe plays short, attenuated clips; it does not change volume, request audio focus, install an APK or modify Reverb.

Omit `--mp3` for a quiet generated tone. `--sharing exclusive` requests exclusive access; results record the actual sharing/performance modes granted. Prefer shared mode for app-compatibility investigations. `--quick` runs only three prepared and three on-demand-file warm trials, with no cold/preopened/idle trials; it takes a few seconds after compilation.

## Modes

- `decode`: independently opens the MP3 and decodes/resamples its first 200 ms. The five repeated preparation timings run before stream setup and are not part of subsequent prepared-audio measurements.
- `cold`: already-decoded PCM exists in memory; timing includes creating/opening the stream and starting it.
- `preopened`: stream is open but stopped; timing starts at requesting playback.
- `warm`: stream is already running silence; timing starts when requesting an existing PCM clip.
- `warm-file`: stream is running silence; timing includes reopening the MP3, creating its decoder, decoding/resampling 200 ms, publishing that PCM and reaching estimated output. The decoder and file are recreated for every sample. Filesystem caches are not flushed.
- `idle`: two seconds of silence after playback, reporting this process's CPU time, callback count and xruns. This excludes audio-service CPU and is not a battery measurement.

## Timing and scope

The output callback marks the first supplied PCM frame using AAudio's frames-written counter. The control thread waits for five distinct, advancing endpoint timestamps at or beyond that frame. It maps each timestamp back to the marked frame at the negotiated 48 kHz rate and reports their median plus spread. A sample fails if timestamps are missing or map before the request/callback. The raw frame/timestamp pairs are retained. No microphones are used: these are endpoint timestamp estimates, not acoustic-onset measurements, and do not account for silence already present in song content.

There is no allocation, mutex, file read or decoding in the audio callback. New clips are published with release/acquire atomics; backing allocations are kept until stream close. This bounded fixture retains all its submitted clips, rather than implementing a production reclamation/ring-buffer scheme. Closing the stream precedes freeing callback state and clips.

`decode.rs` supports stereo MP3 only, using Symphonia 0.5.5 with gapless decoding enabled. It applies a simple linear rate conversion and attenuates the short clip. It does not validate production resampling quality, complete-song decoding, seeking, gapless transitions, changing formats or device disconnection recovery. Dependencies belong only to this standalone workspace; Ink and Reverb dependencies are untouched.

Runs execute as Android shell with direct filesystem access. A production app still needs media URI permission/FD handling, audio focus, session/notification/background lifecycle, route management, and a policy for when to warm or close streams.

[LP3 results](../results/aaudio-lp3-2026-09-29/README.md)
