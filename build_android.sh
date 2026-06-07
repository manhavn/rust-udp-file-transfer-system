#!/bin/bash
# Script build Android client library (client_lib) và APK ở chế độ Debug hoặc Release

# Màu sắc hiển thị
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== UDP Transfer System - Build Script cho Android client_lib & APK ===${NC}"

# Nhận tham số truyền vào (debug hoặc release)
MODE="release"
CARGO_FLAG="--release"
GRADLE_TASK="assembleRelease"
APK_NAME="app-release.apk"
APK_SUBFOLDER="release"

if [ "$1" == "debug" ]; then
    MODE="debug"
    CARGO_FLAG=""
    GRADLE_TASK="assembleDebug"
    APK_NAME="app-debug.apk"
    APK_SUBFOLDER="debug"
    echo -e "Chế độ biên dịch được chọn: ${YELLOW}DEBUG${NC}"
else
    echo -e "Chế độ biên dịch được chọn: ${GREEN}RELEASE (Mặc định)${NC}"
fi

# Đường dẫn Android SDK & NDK
NDK_VERSION="25.2.9519653"
if [ -z "$ANDROID_HOME" ]; then
    export ANDROID_HOME="/home/dev/Android/android-sdk"
fi
export ANDROID_NDK_HOME="${ANDROID_HOME}/ndk/${NDK_VERSION}"

if [ ! -d "$ANDROID_NDK_HOME" ]; then
    echo -e "${RED}Lỗi: Không tìm thấy Android NDK tại: $ANDROID_NDK_HOME${NC}"
    echo -e "Vui lòng cấu hình lại môi trường SDK/NDK."
    exit 1
fi

echo -e "Sử dụng ANDROID_NDK_HOME: ${GREEN}${ANDROID_NDK_HOME}${NC}"

# Danh sách target và thư mục tương ứng
declare -A TARGETS=(
    ["aarch64-linux-android"]="arm64-v8a"
    ["armv7-linux-androideabi"]="armeabi-v7a"
    ["x86_64-linux-android"]="x86_64"
    ["i686-linux-android"]="x86"
)

# Thư mục đích trong Android Project
ANDROID_APP_DIR="$(pwd)/android-app"
ANDROID_JNI_DIR="${ANDROID_APP_DIR}/app/src/main/jniLibs"

# Biên dịch từng target
for target in "${!TARGETS[@]}"; do
    jni_folder="${TARGETS[$target]}"
    echo -e "${BLUE}------------------------------------------------------------${NC}"
    echo -e "Đang biên dịch target Rust: ${YELLOW}${target}${NC} (Cấu hình: ${YELLOW}${MODE}${NC})..."
    
    # Cài đặt target qua rustup nếu chưa có
    rustup target add "$target" &>/dev/null
    
    # Chạy build bằng cargo ndk
    if [ -z "$CARGO_FLAG" ]; then
        cargo ndk -t "$target" build -p client_lib
    else
        cargo ndk -t "$target" build --release -p client_lib
    fi
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✔ Biên dịch thành công target: ${target}${NC}"
        
        # Sao chép vào thư mục jniLibs của Android Project
        mkdir -p "${ANDROID_JNI_DIR}/${jni_folder}"
        cp "target/${target}/${MODE}/libclient_lib.so" "${ANDROID_JNI_DIR}/${jni_folder}/"
        echo -e "Đã copy: target/${target}/${MODE}/libclient_lib.so -> ${ANDROID_JNI_DIR}/${jni_folder}/libclient_lib.so"
    else
        echo -e "${RED}✘ Biên dịch thất bại target: ${target}${NC}"
        exit 1
    fi
done

# Biên dịch APK bằng Gradle
echo -e "${BLUE}------------------------------------------------------------${NC}"
echo -e "Đang tiến hành biên dịch ứng dụng Android APK bằng Gradle..."

if [ -d "$ANDROID_APP_DIR" ]; then
    # Xác định JDK để chạy Gradle (Yêu cầu JDK 17 cho Gradle 8.5)
    export JAVA_HOME="/home/dev/.sdkman/candidates/java/17.0.19-ms"
    export PATH="${JAVA_HOME}/bin:${PATH}"
    
    cd "$ANDROID_APP_DIR"
    ./gradlew "$GRADLE_TASK"
    
    if [ $? -eq 0 ]; then
        echo -e "\n${GREEN}✔ Build Android APK thành công!${NC}"
        echo -e "Đường dẫn APK: ${GREEN}${ANDROID_APP_DIR}/app/build/outputs/apk/${APK_SUBFOLDER}/${APK_NAME}${NC}"
        ls -lh "app/build/outputs/apk/${APK_SUBFOLDER}/${APK_NAME}"
    else
        echo -e "\n${RED}✘ Gradle build thất bại.${NC}"
        exit 1
    fi
else
    echo -e "${RED}Cảnh báo: Không tìm thấy thư mục dự án Android tại: $ANDROID_APP_DIR${NC}"
    echo -e "Không thể copy file .so vào dự án Android."
fi
