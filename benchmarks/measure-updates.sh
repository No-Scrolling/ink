#!/bin/sh
set -eu

repository_root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$repository_root"

if [ -n "$(git status --porcelain --untracked-files=normal)" ]; then
  echo "Commit or stash source changes before building a provenance-checked benchmark." >&2
  exit 1
fi

revision="$(git rev-parse HEAD)"
app="benchmarks/apps/ink-updates"
apk="$app/dist/ink-updates-benchmark-1.0.0-arm64.apk"

mkdir -p "$app/.benchmark"
ln -sf "$HOME/.android/debug.keystore" "$app/.benchmark/debug.keystore"

INK_BENCHMARK=1 \
INK_BENCHMARK_REVISION="$revision" \
INK_KEYSTORE_PASSWORD=android \
INK_KEY_PASSWORD=android \
cargo run --quiet -p ink-cli -- -C "$app" build

apk_sha256="$(shasum -a 256 "$apk" | awk '{ print $1 }')"
INK_BENCHMARK_REVISION="$revision" \
INK_UPDATES_APK_SHA256="$apk_sha256" \
INK_UPDATES_APK="$apk" \
bun benchmarks/measure-updates.ts
