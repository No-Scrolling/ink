#!/bin/sh
set -eu
# Install a published Ink release, or an already verified local release archive.
version="${INK_VERSION:-}"
ink_home="${INK_HOME:-${XDG_DATA_HOME:-$HOME/.local/share}/ink}"
bin_dir="${INK_BIN_DIR:-$HOME/.local/bin}"
green='' reset=''
if [ -t 1 ] && [ "${NO_COLOR+x}" != x ] && [ "${CLICOLOR:-1}" != 0 ] && [ "${TERM:-}" != dumb ]; then
  green=$(printf '\033[32m')
  reset=$(printf '\033[0m')
fi
step() {
  printf '%s %s✓%s %-12s %s\n' "$1" "$green" "$reset" "$2" "$3"
}
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) target=darwin-arm64 ;;
  Linux-x86_64) target=linux-x64 ;;
  Linux-aarch64|Linux-arm64) target=linux-arm64 ;;
  *) echo 'Ink supports macOS Apple Silicon and Linux x64/ARM64.' >&2; exit 1 ;;
esac
for tool in curl tar; do
  command -v "$tool" >/dev/null || { echo "$tool is required" >&2; exit 1; }
done
if command -v sha256sum >/dev/null; then
  checksum=sha256sum
elif command -v shasum >/dev/null; then
  checksum='shasum -a 256'
else
  echo 'sha256sum or shasum is required' >&2
  exit 1
fi
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
asset="ink-$target.tar.gz"
repo=https://github.com/No-Scrolling/ink
printf '\nInk · install\n'
printf '├── %-14s %s\n' 'Platform' "$target"
if [ -n "${INK_ARCHIVE:-}" ]; then
  printf '├── %-14s %s\n' 'Archive' "$INK_ARCHIVE"
  cp "$INK_ARCHIVE" "$tmp/$asset"
  cp "$(dirname "$INK_ARCHIVE")/SHA256SUMS" "$tmp/SHA256SUMS"
else
  # Alpha releases use GitHub prereleases, which releases/latest excludes.
  if [ -z "$version" ]; then
    curl -fsSL 'https://api.github.com/repos/No-Scrolling/ink/releases?per_page=100' -o "$tmp/releases.json"
    version=$(sed -n 's/^[[:space:]]*"tag_name": "\(v[0-9A-Za-z.-]*\)",*$/\1/p' "$tmp/releases.json" | head -n 1)
    if [ -z "$version" ]; then
      echo 'No published Ink release was found.' >&2
      exit 1
    fi
  fi
  case "$version" in v*) ;; *) version="v$version" ;; esac
  printf '├── %-14s %s\n' 'Download' "$version"
  curl -fsSL "$repo/releases/download/$version/$asset" -o "$tmp/$asset"
  curl -fsSL "$repo/releases/download/$version/SHA256SUMS" -o "$tmp/SHA256SUMS"
fi
# A release checksum file covers every platform; only this archive was downloaded.
awk -v asset="$asset" '$2 == asset { print; found = 1 } END { if (!found) exit 1 }' "$tmp/SHA256SUMS" > "$tmp/selected-checksum"
if ! (cd "$tmp" && $checksum -c selected-checksum) > "$tmp/checksum.log" 2>&1; then
  cat "$tmp/checksum.log" >&2
  exit 1
fi
step '├──' 'Verify' 'checksum passed'
mkdir "$tmp/payload"
tar -xzf "$tmp/$asset" -C "$tmp/payload"
release=$("$tmp/payload/bin/bun" -e 'console.log((await Bun.file(process.argv[1]).json()).version)' "$tmp/payload/sdk.json")
case "$release" in *[!0-9A-Za-z.-]*|'') echo 'Invalid release version' >&2; exit 1 ;; esac
"$tmp/payload/bin/ink" --version > /dev/null
step '├──' 'Unpack' "Ink $release"
mkdir -p "$ink_home/versions" "$bin_dir"
if [ -e "$ink_home/versions/$release" ]; then
  echo "Ink $release is already installed at $ink_home/versions/$release" >&2
  exit 1
fi
if [ -e "$bin_dir/ink" ] || [ -L "$bin_dir/ink" ]; then
  if [ "$(readlink "$bin_dir/ink" || true)" != "$ink_home/current/bin/ink" ]; then
    echo "$bin_dir/ink already exists and is not managed by this installer. Choose INK_BIN_DIR." >&2
    exit 1
  fi
fi
mv "$tmp/payload" "$ink_home/versions/$release"
ln -s "$ink_home/versions/$release" "$ink_home/.current-$$"
case "$target" in
  darwin-*) mv -fh "$ink_home/.current-$$" "$ink_home/current" ;;
  linux-*) mv -fT "$ink_home/.current-$$" "$ink_home/current" ;;
esac
ln -sf "$ink_home/current/bin/ink" "$bin_dir/ink"
step '└──' 'Install' "$bin_dir/ink"
printf '\n'
case ":$PATH:" in
  *":$bin_dir:"*) ;;
  *) printf 'Add %s to your PATH to run ink.\n\n' "$bin_dir" ;;
esac
if [ "$target" = linux-arm64 ]; then
  echo 'Android setup requires Linux x64 or macOS Apple Silicon. The CLI is ready to use.'
elif [ -z "${CI:-}" ] && [ "${INK_SKIP_SETUP:-0}" != 1 ] && [ -t 1 ] && ( : < /dev/tty ) 2>/dev/null; then
  # curl | sh uses stdin for the script; setup needs the terminal for its prompts.
  if ! "$bin_dir/ink" setup < /dev/tty; then
    echo 'Ink is installed. Finish the prerequisites above, then run ink setup again.' >&2
  fi
else
  echo 'Run ink setup to check your build tools.'
fi
