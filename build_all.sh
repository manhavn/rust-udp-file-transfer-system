#!/usr/bin/env bash
# Build every release artifact this machine can produce into ./outputs.
#
# Outputs are upload-ready for a GitHub Release: flat files + SHA256SUMS.txt.
# Desktop GUI bundles are skipped when the native Tauri toolchain is unavailable.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"
export PATH="${HOME}/.local/share/mise/shims:${HOME}/.local/bin:${HOME}/.cargo/bin:${PATH}"

PROJECT_NAME="rust-udp-file-transfer-system"
VERSION="${VERSION:-$(sed -n 's/^version = "\(.*\)"$/\1/p' common/Cargo.toml | head -1)}"
[[ -n "$VERSION" ]] || { echo "error: cannot read version from common/Cargo.toml" >&2; exit 1; }
BASE="${PROJECT_NAME}-v${VERSION}"
OUTPUTS_DIR="$ROOT/outputs"

have() { command -v "$1" >/dev/null 2>&1; }
info() { echo "==> $*"; }
warn() { echo "warn: $*" >&2; }

usage() {
  cat <<EOF
Usage: $0 [all|rust|gui|android]...
  all      build every component (default)
  rust     server + client_cli cross-platform archives
  gui      desktop GUI bundles for the current platform
  android  signed Android release APK

Environment:
  VERSION  override the version read from common/Cargo.toml
  TARGETS  comma-separated Rust targets for the rust component
Artifacts: $OUTPUTS_DIR
EOF
}

SELECT_RUST=0 SELECT_GUI=0 SELECT_ANDROID=0
if (( $# == 0 )); then set -- all; fi
for component in "$@"; do
  case "$component" in
    all) SELECT_RUST=1; SELECT_GUI=1; SELECT_ANDROID=1 ;;
    rust|server|cli) SELECT_RUST=1 ;;
    gui) SELECT_GUI=1 ;;
    android) SELECT_ANDROID=1 ;;
    -h|--help|help) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
  esac
done

info "Release ${BASE}"

[[ "$OUTPUTS_DIR" != "/" && "$OUTPUTS_DIR" != "$ROOT" ]] || { echo "error: unsafe outputs path: $OUTPUTS_DIR" >&2; exit 1; }
rm -rf "$OUTPUTS_DIR"
mkdir -p "$OUTPUTS_DIR"

build_rust() {
  info "Building server + client_cli archives"
  local dist="$OUTPUTS_DIR/.rust-dist"
  DIST_DIR="$dist" ./build-cross.sh
  local archive name renamed
  shopt -s nullglob
  for archive in "$dist/packages/"*; do
    [[ -f "$archive" ]] || continue
    [[ "$archive" == *.sha256 ]] && continue
    name="$(basename "$archive")"
    renamed="${name/"$PROJECT_NAME"/"$BASE"}"
    mv -f "$archive" "$OUTPUTS_DIR/$renamed"
  done
  shopt -u nullglob
  rm -rf "$dist"
}

build_gui() {
  if ! have node || ! have npm; then
    warn "GUI skipped: node/npm not installed"
    return 0
  fi
  if [[ "$(uname -s)" == "Linux" ]]; then
    if ! have pkg-config || ! pkg-config --exists webkit2gtk-4.1 gtk+-3.0; then
      warn "GUI skipped: WebKitGTK/GTK development packages not installed"
      return 0
    fi
  fi

  info "Building desktop GUI for this platform"
  (
    cd client_gui
    [[ -d node_modules ]] || npm install
    npm run tauri build
  )

  local bundle="target/release/bundle" file copied=0
  [[ -d "$bundle" ]] || { warn "GUI bundle directory missing: $bundle"; return 0; }
  while IFS= read -r -d '' file; do
    cp -f "$file" "$OUTPUTS_DIR/"
    copied=$((copied + 1))
  done < <(find "$bundle" -type f \( -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' \
           -o -name '*.dmg' -o -name '*.msi' -o -name '*.exe' \) -print0)
  (( copied > 0 )) || warn "GUI build produced no installer bundles"
}

build_android() {
  info "Building signed Android release APK"
  ./build_android.sh release
  local apk="android-app/app/build/outputs/apk/release/app-release.apk"
  [[ -f "$apk" ]] || { echo "error: APK not found: $apk" >&2; exit 1; }
  cp -f "$apk" "$OUTPUTS_DIR/${BASE}-android.apk"
}

(( SELECT_RUST )) && build_rust
(( SELECT_GUI )) && build_gui
(( SELECT_ANDROID )) && build_android

info "Writing SHA256SUMS.txt"
(
  cd "$OUTPUTS_DIR"
  shopt -s nullglob
  files=(*)
  (( ${#files[@]} > 0 )) || { echo "error: no artifacts were produced" >&2; exit 1; }
  sha256sum "${files[@]}" > SHA256SUMS.txt
)

echo
info "Upload-ready artifacts in $OUTPUTS_DIR"
ls -lh "$OUTPUTS_DIR"
