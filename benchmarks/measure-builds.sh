#!/bin/sh
set -eu

output="${BUILD_BENCHMARK_OUTPUT:-benchmarks/results/build.csv}"
rounds="${BUILD_BENCHMARK_ROUNDS:-3}"
stacks=" $(printf '%s' "${BUILD_BENCHMARK_STACKS:-ink,expo,light}" | tr ',' ' ') "
light_sdk_dir="${LIGHT_SDK_DIR:-$HOME/Developer/light-sdk}"

case "$stacks" in
  *" expo "*)
    : "${EXPO_COUNTER_DIR:?Set EXPO_COUNTER_DIR to the prepared Expo counter project}"
    : "${EXPO_SCROLL_DIR:?Set EXPO_SCROLL_DIR to the prepared Expo scroll project}"
    ;;
esac
mkdir -p "$(dirname "$output")"
mkdir -p benchmarks/apps/ink-counter/.benchmark benchmarks/apps/ink-scroll/.benchmark
ln -sf "$HOME/.android/debug.keystore" benchmarks/apps/ink-counter/.benchmark/debug.keystore
ln -sf "$HOME/.android/debug.keystore" benchmarks/apps/ink-scroll/.benchmark/debug.keystore
echo "round,stack,scenario,kind,seconds" > "$output"

measure() {
  round="$1"
  stack="$2"
  scenario="$3"
  kind="$4"
  shift 4
  started="$(perl -MTime::HiRes=time -e 'printf "%.6f", time')"
  "$@" >/dev/null
  finished="$(perl -MTime::HiRes=time -e 'printf "%.6f", time')"
  seconds="$(awk -v started="$started" -v finished="$finished" 'BEGIN { printf "%.3f", finished - started }')"
  echo "$round,$stack,$scenario,$kind,$seconds" >> "$output"
  echo "$round/$rounds $stack $scenario $kind: ${seconds}s"
}

clean_ink() {
  capabilities="$(pwd)/benchmarks/empty-capabilities.json"
  ./platform/android/gradlew -p platform/android :app:clean --quiet \
    -PinkCapabilitiesManifest="$capabilities" >/dev/null
}

build_ink() {
  app="$1"
  INK_KEYSTORE_PASSWORD=android INK_KEY_PASSWORD=android ink -C "$app" build
}

clean_expo() {
  project="$1"
  "$project/android/gradlew" -p "$project/android" :app:clean --quiet >/dev/null
}

build_expo() {
  project="$1"
  NODE_ENV=production "$project/android/gradlew" -p "$project/android" :app:assembleRelease --no-daemon -PreactNativeArchitectures=arm64-v8a --quiet
}

clean_light_sdk() {
  module="$1"
  "$light_sdk_dir/gradlew" -p "$light_sdk_dir" ":$module:clean" --quiet >/dev/null
}

build_light_sdk() {
  module="$1"
  "$light_sdk_dir/gradlew" -p "$light_sdk_dir" ":$module:assembleRelease" --no-daemon --quiet
}

case "$stacks" in
  *" ink "*) build_ink benchmarks/apps/ink-counter >/dev/null ;;
esac

round=1
while [ "$round" -le "$rounds" ]; do
  if [ $((round % 2)) -eq 1 ]; then
    order="ink expo light"
  else
    order="light expo ink"
  fi

  for stack in $order; do
    case "$stacks" in
      *" $stack "*) ;;
      *) continue ;;
    esac
    case "$stack" in
      ink)
        clean_ink
        measure "$round" Ink Counter clean build_ink benchmarks/apps/ink-counter
        clean_ink
        measure "$round" Ink Scroll clean build_ink benchmarks/apps/ink-scroll
        measure "$round" Ink Scroll noop build_ink benchmarks/apps/ink-scroll
        ;;
      expo)
        clean_expo "$EXPO_COUNTER_DIR"
        measure "$round" Expo Counter clean build_expo "$EXPO_COUNTER_DIR"
        clean_expo "$EXPO_SCROLL_DIR"
        measure "$round" Expo Scroll clean build_expo "$EXPO_SCROLL_DIR"
        measure "$round" Expo Scroll noop build_expo "$EXPO_SCROLL_DIR"
        ;;
      light)
        clean_light_sdk benchmark-counter
        measure "$round" Light-SDK Counter clean build_light_sdk benchmark-counter
        clean_light_sdk benchmark-scroll
        measure "$round" Light-SDK Scroll clean build_light_sdk benchmark-scroll
        measure "$round" Light-SDK Scroll noop build_light_sdk benchmark-scroll
        ;;
    esac
  done
  round=$((round + 1))
done

echo "Wrote $output"
