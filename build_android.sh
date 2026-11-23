#!/usr/bin/env bash
# Build Android client library (client_lib) + APK ở chế độ Debug hoặc Release.
# Tự dò/cài SDK, NDK, JDK, cargo-ndk và Rust targets nếu máy chưa có.
# Ưu tiên dùng mise cho JDK/cargo-ndk khi mise có sẵn.

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

# Đưa mise shims / cargo / các binary user-local vào PATH.
export PATH="${HOME}/.local/share/mise/shims:${HOME}/.local/bin:${HOME}/.cargo/bin:${PATH}"

have() { command -v "$1" >/dev/null 2>&1; }
info() { echo -e "${BLUE}$*${NC}"; }
ok()   { echo -e "${GREEN}$*${NC}"; }
warn() { echo -e "${YELLOW}$*${NC}"; }
die()  { echo -e "${RED}Lỗi: $*${NC}" >&2; exit 1; }

usage() {
    echo "Cách dùng: $0 [debug|release]"
    echo "  debug   : build APK debug (assembleDebug)"
    echo "  release : build APK release đã ký (assembleRelease) - mặc định"
}

MODE="release"
GRADLE_TASK="assembleRelease"
APK_NAME="app-release.apk"
APK_SUBFOLDER="release"
case "${1:-release}" in
    debug)   MODE="debug";   GRADLE_TASK="assembleDebug";   APK_NAME="app-debug.apk";   APK_SUBFOLDER="debug" ;;
    release) ;;
    -h|--help|help) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
esac

echo -e "${BLUE}=== UDP Transfer System - Build Script cho Android client_lib & APK ===${NC}"
echo -e "Chế độ biên dịch: ${YELLOW}${MODE^^}${NC}"

# ---------------------------------------------------------------------------
# 1. Android SDK + command-line tools
# ---------------------------------------------------------------------------
find_sdk() {
    local candidate
    for candidate in "${ANDROID_HOME:-}" "${ANDROID_SDK_ROOT:-}" \
                     "${HOME}/Android/Sdk" "${HOME}/Android/android-sdk" \
                     "${HOME}/android-sdk" /opt/android-sdk /usr/lib/android-sdk; do
        if [[ -n "$candidate" && -d "$candidate" ]]; then
            echo "$candidate"
            return 0
        fi
    done
    return 1
}

install_sdk() {
    local sdk="$1"
    local url="${CMDLINE_TOOLS_URL:-https://dl.google.com/android/repository/commandlinetools-linux-13114758_latest.zip}"
    local tmp
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT

    info "Không tìm thấy Android SDK. Tải command-line tools vào ${sdk}..."
    mkdir -p "$sdk/cmdline-tools"
    if have curl; then
        curl -fL --retry 3 -o "$tmp/cmdline-tools.zip" "$url"
    elif have wget; then
        wget -O "$tmp/cmdline-tools.zip" "$url"
    else
        die "Cần curl hoặc wget để tải Android command-line tools."
    fi
    have unzip || die "Cần unzip để giải nén Android command-line tools."
    unzip -q "$tmp/cmdline-tools.zip" -d "$tmp"
    rm -rf "$sdk/cmdline-tools/latest"
    mv "$tmp/cmdline-tools" "$sdk/cmdline-tools/latest"
    rm -rf "$tmp"
    trap - EXIT
}

SDK="$(find_sdk || true)"
if [[ -z "$SDK" ]]; then
    SDK="${HOME}/Android/Sdk"
    install_sdk "$SDK"
fi
export ANDROID_HOME="$SDK"
export ANDROID_SDK_ROOT="$SDK"
ok "Android SDK: $ANDROID_HOME"

sdkmanager_path() {
    local candidate
    for candidate in "$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" \
                     "$ANDROID_HOME/tools/bin/sdkmanager"; do
        [[ -x "$candidate" ]] && { echo "$candidate"; return 0; }
    done
    candidate="$(find "$ANDROID_HOME/cmdline-tools" -maxdepth 3 -name sdkmanager -type f 2>/dev/null | head -1)"
    [[ -n "$candidate" ]] && { echo "$candidate"; return 0; }
    return 1
}

SDKMANAGER="$(sdkmanager_path || true)"
[[ -n "$SDKMANAGER" ]] || die "Không tìm thấy sdkmanager trong $ANDROID_HOME/cmdline-tools. Hãy cài Android command-line tools."

# ---------------------------------------------------------------------------
# 2. JDK (AGP 8.2 / Gradle 8.5 cần JDK 17-21)
# ---------------------------------------------------------------------------
java_major() {
    local bin="$1/bin/java"
    [[ -x "$bin" ]] || return 1
    "$bin" -version 2>&1 | awk -F'"' '/version/ { split($2, v, "."); print (v[1] == "1" ? v[2] : v[1]); exit }'
}

resolve_java() {
    local candidate major
    for candidate in "${JAVA_HOME:-}" "$(mise where java 2>/dev/null || true)"; do
        [[ -n "$candidate" && -x "$candidate/bin/java" ]] || continue
        major="$(java_major "$candidate" || true)"
        if [[ -n "$major" ]] && (( major >= 17 && major <= 21 )); then
            echo "$candidate"
            return 0
        fi
    done
    return 1
}

JDK="$(resolve_java || true)"
if [[ -z "$JDK" ]] && have mise; then
    info "Cài JDK 21 qua mise..."
    mise install java >/dev/null 2>&1 || mise install java@21 >/dev/null 2>&1 || true
    JDK="$(resolve_java || true)"
fi
if [[ -z "$JDK" ]] && have mise; then
    info "Thử cài JDK 17 qua mise..."
    mise install java@17 >/dev/null 2>&1 || true
    JDK="$(mise where java@17 2>/dev/null || true)"
    [[ -x "$JDK/bin/java" ]] || JDK=""
fi
[[ -n "$JDK" ]] || die "Không tìm thấy JDK 17-21. Cài bằng 'mise install java@21' hoặc đặt JAVA_HOME."
export JAVA_HOME="$JDK"
export PATH="$JAVA_HOME/bin:$PATH"
ok "JDK: $JAVA_HOME ($(java_major "$JAVA_HOME"))"

# ---------------------------------------------------------------------------
# 3. SDK packages cần cho compileSdk/build-tools
# ---------------------------------------------------------------------------
COMPILE_SDK="$(sed -n 's/^[[:space:]]*compileSdk\(Version\)\?[[:space:]]*\([0-9][0-9]*\).*/\2/p' android-app/app/build.gradle | head -1)"
COMPILE_SDK="${COMPILE_SDK:-34}"
BUILD_TOOLS_VERSION="${BUILD_TOOLS_VERSION:-34.0.0}"

MISSING_PKGS=()
[[ -d "$ANDROID_HOME/platforms/android-${COMPILE_SDK}" ]] || MISSING_PKGS+=("platforms;android-${COMPILE_SDK}")
[[ -d "$ANDROID_HOME/build-tools/${BUILD_TOOLS_VERSION}" ]] || MISSING_PKGS+=("build-tools;${BUILD_TOOLS_VERSION}")
[[ -d "$ANDROID_HOME/platform-tools" ]] || MISSING_PKGS+=("platform-tools")

if (( ${#MISSING_PKGS[@]} > 0 )); then
    info "Cài SDK packages còn thiếu: ${MISSING_PKGS[*]}"
    yes | "$SDKMANAGER" --licenses >/dev/null 2>&1 || true
    "$SDKMANAGER" --sdk_root="$ANDROID_HOME" --install "${MISSING_PKGS[@]}"
else
    ok "SDK packages: platform android-${COMPILE_SDK}, build-tools ${BUILD_TOOLS_VERSION}"
fi

# ---------------------------------------------------------------------------
# 4. NDK (ưu tiên NDK đang cài, nếu chưa có thì tải bản pin)
# ---------------------------------------------------------------------------
NDK_VERSION="${NDK_VERSION:-28.2.13676358}"

find_ndk() {
    local candidate
    for candidate in "${ANDROID_NDK_HOME:-}" "${ANDROID_NDK_ROOT:-}" "$ANDROID_HOME/ndk/${NDK_VERSION}"; do
        [[ -n "$candidate" && -d "$candidate/toolchains/llvm" ]] && { echo "$candidate"; return 0; }
    done
    if [[ -d "$ANDROID_HOME/ndk" ]]; then
        while IFS= read -r candidate; do
            [[ -d "$ANDROID_HOME/ndk/$candidate/toolchains/llvm" ]] && { echo "$ANDROID_HOME/ndk/$candidate"; return 0; }
        done < <(ls -1 "$ANDROID_HOME/ndk" | sort -Vr)
    fi
    return 1
}

NDK_HOME="$(find_ndk || true)"
if [[ -z "$NDK_HOME" ]]; then
    info "Chưa có NDK. Tải NDK ${NDK_VERSION} qua sdkmanager..."
    yes | "$SDKMANAGER" --licenses >/dev/null 2>&1 || true
    "$SDKMANAGER" --sdk_root="$ANDROID_HOME" --install "ndk;${NDK_VERSION}"
    NDK_HOME="$ANDROID_HOME/ndk/${NDK_VERSION}"
fi
[[ -d "$NDK_HOME/toolchains/llvm" ]] || die "NDK không hợp lệ: $NDK_HOME"
export ANDROID_NDK_HOME="$NDK_HOME"
export ANDROID_NDK_ROOT="$NDK_HOME"
ok "NDK: $ANDROID_NDK_HOME"

# ---------------------------------------------------------------------------
# 5. cargo-ndk + Rust targets
# ---------------------------------------------------------------------------
if ! have cargo-ndk; then
    if have mise; then
        info "Cài cargo-ndk qua mise..."
        mise install cargo:cargo-ndk@latest >/dev/null 2>&1 || true
        MISE_NDK="$(mise where cargo:cargo-ndk@latest 2>/dev/null || true)"
        [[ -n "$MISE_NDK" ]] && export PATH="$MISE_NDK/bin:$PATH"
    fi
fi
have cargo-ndk || cargo install cargo-ndk --locked
have cargo-ndk || die "Không cài được cargo-ndk."

have rustup || die "Cần rustup để thêm Rust targets. Cài Rust qua mise/rustup trước."

ABI_TARGETS=(
    "arm64-v8a:aarch64-linux-android"
    "armeabi-v7a:armv7-linux-androideabi"
    "x86_64:x86_64-linux-android"
    "x86:i686-linux-android"
)

CARGO_ARGS=(build)
[[ "$MODE" == "release" ]] && CARGO_ARGS+=(--release)

ANDROID_JNI_DIR="$ROOT/android-app/app/src/main/jniLibs"

for pair in "${ABI_TARGETS[@]}"; do
    ABI="${pair%%:*}"
    TARGET="${pair##*:}"
    echo -e "${BLUE}------------------------------------------------------------${NC}"
    info "Biên dịch Rust target ${TARGET} (${ABI}, ${MODE})..."

    rustup target add "$TARGET" >/dev/null

    cargo ndk -t "$ABI" "${CARGO_ARGS[@]}" -p client_lib

    SO_FILE="$ROOT/target/${TARGET}/${MODE}/libclient_lib.so"
    [[ -f "$SO_FILE" ]] || die "Không thấy output: $SO_FILE"
    mkdir -p "$ANDROID_JNI_DIR/$ABI"
    cp -f "$SO_FILE" "$ANDROID_JNI_DIR/$ABI/libclient_lib.so"
    ok "✔ ${ABI}/libclient_lib.so"
done

# ---------------------------------------------------------------------------
# 6. Gradle -> APK
# ---------------------------------------------------------------------------
info "Ghi android-app/local.properties và build APK bằng Gradle..."
printf 'sdk.dir=%s\n' "$ANDROID_HOME" > "$ROOT/android-app/local.properties"

[[ -d "$ROOT/android-app" ]] || die "Không tìm thấy thư mục android-app."

cd "$ROOT/android-app"
./gradlew "$GRADLE_TASK" --no-daemon

APK_PATH="$ROOT/android-app/app/build/outputs/apk/${APK_SUBFOLDER}/${APK_NAME}"
[[ -f "$APK_PATH" ]] || die "Gradle báo thành công nhưng không thấy APK: $APK_PATH"

echo
ok "✔ Build Android APK thành công!"
echo -e "APK: ${GREEN}${APK_PATH}${NC}"
ls -lh "$APK_PATH"
