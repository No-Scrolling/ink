# Full Android SDK contracts

Run `bun run test android-app --serial emulator-5554`.

The runner builds a genuine development app through the Ink CLI, installs the unique `com.vandam.ink.sdkcontracts` package and launches the production activity. Public SDK operations travel through compiled JavaScript, QuickJS, Rust/JNI and the actual Kotlin adapters. Observations return through production console messages in PID-filtered logcat; assertions live independently in the host runner.

Eleven contracts cover committed schema migration, competing SDK updates, sibling subscription refresh, rejection of older schema readers, persistence without repeated migration after a process restart, binary HTTP upload/response bytes, 307 replay, 303 method/body rewriting, incremental response reading through EOF, body cancellation with subsequent request recovery, and rejection of a pre-aborted request before it reaches the server.

The controlled HTTP peer is a real local Bun server. Its expected request bytes are explicit, and response bytes are authored separately from the fixture's observations. The runner maps device loopback port 18765 to a dynamic host port, refuses an existing mapping or fixture installation, and uses the same per-device lock as the adapter suite. Request records, both process observations and build/device logs remain in ignored `.test-output/android-app/`.

The fixture APK, reverse mapping, local server and lock are removed in `finally`. Build caches and logs remain ignored. A forcibly terminated runner may require explicit cleanup before retrying. The runner does not start or stop the emulator.

This exercises development-app contracts. It does not establish release network policy, cross-process subscriptions, microphone/camera behaviour, physical Light Phone performance or renderer pixels. Stream tests verify content and EOF, without depending on network chunk boundaries. The concurrent-update test checks preservation of both writes without promising that a particular run must force a CAS conflict; the adapter suite separately tests competing CAS attempts.
