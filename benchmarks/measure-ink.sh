#!/bin/sh
set -eu

build_output="${BUILD_BENCHMARK_OUTPUT:-benchmarks/results/ink-build.csv}"
BUILD_BENCHMARK_STACKS=ink \
  BUILD_BENCHMARK_OUTPUT="$build_output" \
  ./benchmarks/measure-builds.sh

adb="${ADB:-$HOME/Library/Android/sdk/platform-tools/adb}"
benchmark_device="${BENCHMARK_DEVICE:-$("$adb" devices | awk '/^emulator-/{ print $1; exit }')}"
benchmark_output="${BENCHMARK_OUTPUT:-benchmarks/results/ink.json}"
if [ -z "$benchmark_device" ]; then
  echo "No benchmark device found. Start an emulator or set BENCHMARK_DEVICE." >&2
  exit 1
fi

BENCHMARK_STACKS=ink \
  BENCHMARK_DEVICE="$benchmark_device" \
  BENCHMARK_OUTPUT="$benchmark_output" \
  bun benchmarks/measure.ts
if [ -n "${INK_BENCHMARK_BUDGETS:-}" ]; then
  bun benchmarks/verify.ts "$benchmark_output" "$build_output"
fi
