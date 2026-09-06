#!/bin/sh
set -eu

output="${BUILD_BENCHMARK_OUTPUT:-benchmarks/results/build.csv}"
rounds="${BUILD_BENCHMARK_ROUNDS:-3}"
stacks=" $(printf '%s' "${BUILD_BENCHMARK_STACKS:-ink,expo,light}" | tr ',' ' ') "
light_sdk_dir="${LIGHT_SDK_DIR:-$HOME/Developer/light-sdk}"

case "$stacks" in
  *" expo "*)
    : "${EXPO_COUNTER_DIR:?Set EXPO_COUNTER_DIR to the prepared Expo counter project}"
    ;;
esac
mkdir -p "$(dirname "$output")"
mkdir -p benchmarks/apps/ink-counter/.benchmark
ln -sf "$HOME/.android/debug.keystore" benchmarks/apps/ink-counter/.benchmark/debug.keystore
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
  app="$(cd "$1" && pwd)"
  capabilities="$(pwd)/benchmarks/empty-capabilities.json"
  ./platform/android/gradlew -p platform/android :app:clean --quiet \
    --project-cache-dir "$app/.ink/build/gradle-cache" \
    -PinkBuildRoot="$app/.ink/build/android" \
    -PinkCapabilitiesManifest="$capabilities" >/dev/null
}

build_ink() {
  app="$1"
  INK_KEYSTORE_PASSWORD=android INK_KEY_PASSWORD=android ./scripts/ink -C "$app" build
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
        clean_ink benchmarks/apps/ink-counter
        measure "$round" Ink Counter clean build_ink benchmarks/apps/ink-counter
        measure "$round" Ink Counter noop build_ink benchmarks/apps/ink-counter
        ;;
      expo)
        clean_expo "$EXPO_COUNTER_DIR"
        measure "$round" Expo Counter clean build_expo "$EXPO_COUNTER_DIR"
        measure "$round" Expo Counter noop build_expo "$EXPO_COUNTER_DIR"
        ;;
      light)
        clean_light_sdk benchmark-counter
        measure "$round" Light-SDK Counter clean build_light_sdk benchmark-counter
        measure "$round" Light-SDK Counter noop build_light_sdk benchmark-counter
        ;;
    esac
  done
  round=$((round + 1))
done

echo "Wrote $output"
