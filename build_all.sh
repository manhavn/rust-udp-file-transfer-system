#!/usr/bin/env bash
# Build every release artifact this machine can produce into ./outputs.
#
# Artifacts are copied into outputs/ as each component finishes, so a failing
# component does not discard artifacts that were built successfully. Pass
# --clean to start from an empty outputs directory.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"
export PATH="${HOME}/.local/share/mise/shims:${HOME}/.local/bin:${HOME}/.cargo/bin:${PATH}"

PROJECT_NAME="rust-udp-file-transfer-system"
VERSION="${VERSION:-$(sed -n 's/^version = "\(.*\)"$/\1/p' common/Cargo.toml | head -1)}"
[[ -n "$VERSION" ]] || { echo "error: cannot read version from common/Cargo.toml" >&2; exit 1; }
BASE="${PROJECT_NAME}-v${VERSION}"
OUTPUTS_DIR="$ROOT/outputs"
STAGE="$(mktemp -d "${TMPDIR:-/tmp}/rtk-build-all.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT

have() { command -v "$1" >/dev/null 2>&1; }
info() { echo "==> $*"; }
warn() { echo "warn: $*" >&2; }

usage() {
  cat <<EOF
Usage: $0 [--clean] [all|rust|gui|android]...
  all      build every component (default)
  rust     server + client_cli cross-platform archives
  gui      desktop GUI bundles for the current platform
  android  signed Android release APK

Options:
  --clean  remove outputs/ before building

Environment:
  VERSION  override the version read from common/Cargo.toml
  TARGETS  comma-separated Rust targets for the rust component
Artifacts: $OUTPUTS_DIR
EOF
}

SELECT_RUST=0 SELECT_GUI=0 SELECT_ANDROID=0 CLEAN=0
if (( $# == 0 )); then set -- all; fi
for component in "$@"; do
  case "$component" in
    all) SELECT_RUST=1; SELECT_GUI=1; SELECT_ANDROID=1 ;;
    rust|server|cli) SELECT_RUST=1 ;;
    gui) SELECT_GUI=1 ;;
    android) SELECT_ANDROID=1 ;;
    --clean) CLEAN=1 ;;
    -h|--help|help) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
  esac
done
if (( SELECT_RUST + SELECT_GUI + SELECT_ANDROID == 0 )); then
  SELECT_RUST=1; SELECT_GUI=1; SELECT_ANDROID=1
fi

info "Release ${BASE}"

[[ "$OUTPUTS_DIR" != "/" && "$OUTPUTS_DIR" != "$ROOT" ]] || { echo "error: unsafe outputs path: $OUTPUTS_DIR" >&2; exit 1; }
if (( CLEAN )); then
  rm -rf "$OUTPUTS_DIR"
fi
mkdir -p "$OUTPUTS_DIR"

collect_rust() {
  local dist="$1" archive name renamed
  shopt -s nullglob
  for archive in "$dist/packages/"*; do
    [[ -f "$archive" ]] || continue
    [[ "$archive" == *.sha256 ]] && continue
    name="$(basename "$archive")"
    renamed="${name/"$PROJECT_NAME"/"$BASE"}"
    cp -f "$archive" "$OUTPUTS_DIR/$renamed"
  done
  shopt -u nullglob
}

build_rust() {
  info "Building server + client_cli archives"
  local dist="$STAGE/rust" rc=0
  # build-cross stops at the first failing target; copy whatever it produced.
  DIST_DIR="$dist" ./build-cross.sh || rc=$?
  collect_rust "$dist"
  return "$rc"
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
  rm -rf "$ROOT/target/release/bundle"
  local rc=0
  (
    cd client_gui
    [[ -d node_modules ]] || npm install
    # Tauri patches the binary in place, and cargo hardlinks it from deps/, so
    # rebuild client_gui to hand Tauri an unstamped binary on every run.
    cargo clean -p client_gui --release
    npm run tauri build
  ) || rc=$?

  local bundle="target/release/bundle" file copied=0
  if [[ -d "$bundle" ]]; then
    while IFS= read -r -d '' file; do
      cp -f "$file" "$OUTPUTS_DIR/"
      copied=$((copied + 1))
    done < <(find "$bundle" -type f \( -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' \
             -o -name '*.dmg' -o -name '*.msi' -o -name '*.exe' \) -print0)
  fi
  (( copied > 0 )) || warn "GUI build produced no installer bundles"
  return "$rc"
}

build_android() {
  info "Building signed Android release APK"
  ./build_android.sh release
  local apk="android-app/app/build/outputs/apk/release/app-release.apk"
  [[ -f "$apk" ]] || { echo "error: APK not found: $apk" >&2; return 1; }
  cp -f "$apk" "$OUTPUTS_DIR/${BASE}-android.apk"
}

FAILED=()
if (( SELECT_RUST )); then build_rust || FAILED+=("rust"); fi
if (( SELECT_GUI )); then build_gui || FAILED+=("gui"); fi
if (( SELECT_ANDROID )); then build_android || FAILED+=("android"); fi

info "Writing SHA256SUMS.txt"
(
  cd "$OUTPUTS_DIR"
  shopt -s nullglob
  files=()
  for file in *; do
    [[ -f "$file" && "$file" != SHA256SUMS.txt ]] && files+=("$file")
  done
  (( ${#files[@]} > 0 )) || { echo "error: no artifacts were produced" >&2; exit 1; }
  sha256sum "${files[@]}" > SHA256SUMS.tmp
  mv -f SHA256SUMS.tmp SHA256SUMS.txt
)

echo
info "Upload-ready artifacts in $OUTPUTS_DIR"
ls -lh "$OUTPUTS_DIR"

if (( ${#FAILED[@]} > 0 )); then
  warn "failed component(s): ${FAILED[*]}; artifacts that did build are in $OUTPUTS_DIR"
  exit 1
fi
