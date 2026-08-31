#!/bin/sh
set -eu

BUILD_BENCHMARK_STACKS=ink \
  BUILD_BENCHMARK_OUTPUT=benchmarks/results/ink-build.csv \
  ./benchmarks/measure-builds.sh

adb="${ADB:-$HOME/Library/Android/sdk/platform-tools/adb}"
benchmark_device="${BENCHMARK_DEVICE:-$("$adb" devices | awk '/^emulator-/{ print $1; exit }')}"
if [ -z "$benchmark_device" ]; then
  echo "No Android emulator found" >&2
  exit 1
fi

BENCHMARK_STACKS=ink \
  BENCHMARK_DEVICE="$benchmark_device" \
  BENCHMARK_OUTPUT=benchmarks/results/ink.json \
  bun benchmarks/measure.ts
bun benchmarks/verify.ts \
  benchmarks/results/ink.json \
  benchmarks/results/ink-build.csv
