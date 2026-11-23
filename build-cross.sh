#!/usr/bin/env bash
set -euo pipefail

# Standalone release builder for the server and CLI client.
# GUI, Android, and client library packages retain their dedicated builders.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"
export PATH="${HOME}/.local/bin:${HOME}/.cargo/bin:${PATH}"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-11.0}"

PROJECT_NAME="rust-udp-file-transfer-system"
BIN_NAMES=("server" "client_cli")
DEFAULT_TARGETS="x86_64-unknown-linux-gnu,x86_64-unknown-linux-musl,aarch64-unknown-linux-gnu,aarch64-unknown-linux-musl,x86_64-apple-darwin,aarch64-apple-darwin,x86_64-pc-windows-gnu"
TARGETS_CSV="${TARGETS:-$DEFAULT_TARGETS}"

have() { command -v "$1" >/dev/null 2>&1; }
fail() { echo "error: $*" >&2; exit 1; }
have cargo-zigbuild || fail "cargo-zigbuild is required (cargo install cargo-zigbuild --locked)"
have zig || fail "Zig is required"
have rustup || fail "rustup is required"

if [[ "$TARGETS_CSV" == *apple-darwin* && -z "${SDKROOT:-}" ]]; then
  for sdk in "${HOME}/.sdk/MacOSX11.3.sdk" "${HOME}/.local/macos-sdk/MacOSX11.3.sdk" "${HOME}/.local/macos-sdk/MacOSX12.3.sdk" "${HOME}/.local/macos-sdk/MacOSX13.3.sdk" "${HOME}/.local/macos-sdk/MacOSX14.0.sdk"; do
    if [[ -d "$sdk" ]]; then export SDKROOT="$sdk"; break; fi
  done
fi
if [[ "$TARGETS_CSV" == *apple-darwin* ]]; then
  [[ -d "${SDKROOT:-}" ]] || fail "Apple SDK not found; set SDKROOT=/path/to/MacOSX.sdk"
fi

cargo fmt --all --check
DIST_DIR="${DIST_DIR:-$ROOT/dist}"
[[ "$DIST_DIR" != "/" && "$DIST_DIR" != "$ROOT" ]] || fail "unsafe dist path: $DIST_DIR"
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR/packages"

package_target() {
  local target="$1" target_dist="$DIST_DIR/$1" archive
  if [[ "$target" == *windows* ]] && have zip; then
    archive="$DIST_DIR/packages/${PROJECT_NAME}-${target}.zip"
    (cd "$target_dist" && zip -q -9 "$archive" ./*.exe)
  else
    archive="$DIST_DIR/packages/${PROJECT_NAME}-${target}.tar.gz"
    tar -C "$target_dist" -czf "$archive" .
  fi
  (cd "$DIST_DIR/packages" && sha256sum "$(basename "$archive")" > "$(basename "$archive").sha256")
  echo "    $archive"
}

IFS=',' read -r -a BUILD_TARGETS <<< "$TARGETS_CSV"
built=()
for target in "${BUILD_TARGETS[@]}"; do
  target="$(echo "$target" | xargs)"
  [[ -n "$target" ]] || continue
  echo "==> Building $PROJECT_NAME for $target"
  rustup target add "$target" >/dev/null
  SDKROOT="${SDKROOT:-}" cargo zigbuild --release --target "$target" -p server -p client_cli
  target_dist="$DIST_DIR/$target"
  mkdir -p "$target_dist"
  ext=""; [[ "$target" == *windows* ]] && ext=".exe"
  for bin in "${BIN_NAMES[@]}"; do
    src="$ROOT/target/$target/release/${bin}${ext}"
    [[ -f "$src" ]] || fail "binary not found: $src"
    cp -f "$src" "$target_dist/"
  done
  package_target "$target"
  built+=("$target")
done

if [[ -d "$DIST_DIR/x86_64-apple-darwin" && -d "$DIST_DIR/aarch64-apple-darwin" ]]; then
  lipo_cmd=""; have llvm-lipo && lipo_cmd="llvm-lipo"; [[ -z "$lipo_cmd" ]] && have lipo && lipo_cmd="lipo"
  if [[ -n "$lipo_cmd" ]]; then
    universal_dir="$DIST_DIR/universal-apple-darwin"
    mkdir -p "$universal_dir"
    for bin in "${BIN_NAMES[@]}"; do
      "$lipo_cmd" -create "$DIST_DIR/x86_64-apple-darwin/$bin" "$DIST_DIR/aarch64-apple-darwin/$bin" -output "$universal_dir/$bin"
      chmod +x "$universal_dir/$bin"
    done
    package_target "universal-apple-darwin"
    built+=("universal-apple-darwin")
  else
    echo "warn: universal macOS skipped; install llvm-lipo or run on macOS" >&2
  fi
fi

echo "Built: ${built[*]}"
echo "Upload-ready files: $DIST_DIR/packages/"

