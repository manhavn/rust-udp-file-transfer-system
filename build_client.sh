#!/bin/bash
# Script build client truyền file (client_cli) trên nhiều môi trường OS và Architecture

# Màu sắc hiển thị
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== UDP Transfer System - Build Script cho Client CLI ===${NC}"
echo "Vui lòng chọn hệ điều hành và kiến trúc đích để biên dịch:"
echo "1) Hệ điều hành hiện tại (Native)"
echo "2) Linux x86_64 (Tiêu chuẩn 64-bit)"
echo "3) Linux aarch64 (ARM64 - Raspberry Pi, thiết bị ARM)"
echo "4) Windows x86_64 (Tải file thực thi .exe cho Windows)"
echo "5) macOS x86_64 (Cho máy Mac chip Intel)"
echo "6) macOS aarch64 (Cho máy Mac chip Apple Silicon M1/M2/M3)"
echo "7) Thoát"

read -p "Nhập lựa chọn của bạn (1-7): " choice

case $choice in
    1)
        TARGET=""
        TARGET_DESC="Hệ điều hành hiện tại (Native)"
        ;;
    2)
        TARGET="x86_64-unknown-linux-gnu"
        TARGET_DESC="Linux x86_64"
        ;;
    3)
        TARGET="aarch64-unknown-linux-gnu"
        TARGET_DESC="Linux aarch64 (ARM64)"
        ;;
    4)
        TARGET="x86_64-pc-windows-gnu"
        TARGET_DESC="Windows x86_64 (.exe)"
        ;;
    5)
        TARGET="x86_64-apple-darwin"
        TARGET_DESC="macOS x86_64 (Intel)"
        ;;
    6)
        TARGET="aarch64-apple-darwin"
        TARGET_DESC="macOS aarch64 (Apple Silicon)"
        ;;
    *)
        echo -e "${YELLOW}Đã thoát.${NC}"
        exit 0
        ;;
esac

echo -e "\n${YELLOW}Đang chuẩn bị biên dịch cho: ${TARGET_DESC}...${NC}"

# Kiểm tra và thêm target bằng rustup nếu chọn cross-compile
if [ -n "$TARGET" ]; then
    echo -e "${BLUE}[1/2] Đang kiểm tra và cài đặt target: $TARGET...${NC}"
    rustup target add $TARGET
    if [ $? -ne 0 ]; then
        echo -e "${RED}Lỗi: Không thể thêm target $TARGET bằng rustup. Vui lòng kiểm tra kết nối mạng hoặc phiên bản Rust.${NC}"
        exit 1
    fi
fi

echo -e "${BLUE}[2/2] Đang biên dịch client_cli ở chế độ Release...${NC}"
if [ -z "$TARGET" ]; then
    cargo build --release --bin client_cli
    BUILD_STATUS=$?
    OUT_DIR="target/release"
    BINARY_NAME="client_cli"
else
    cargo build --release --target $TARGET --bin client_cli
    BUILD_STATUS=$?
    OUT_DIR="target/$TARGET/release"
    if [[ "$TARGET" == *"windows"* ]]; then
        BINARY_NAME="client_cli.exe"
    else
        BINARY_NAME="client_cli"
    fi
fi

if [ $BUILD_STATUS -eq 0 ]; then
    echo -e "\n${GREEN}✔ Biên dịch thành công!${NC}"
    echo -e "File thực thi được lưu tại: ${GREEN}${OUT_DIR}/${BINARY_NAME}${NC}"
else
    echo -e "\n${RED}✘ Biên dịch thất bại.${NC}"
    if [[ "$TARGET" == *"windows"* ]]; then
        echo -e "${YELLOW}Lưu ý: Để build cho Windows trên Linux, bạn cần cài đặt bộ biên dịch chéo: sudo apt install mingw-w64${NC}"
    elif [[ "$TARGET" == *"darwin"* ]]; then
        echo -e "${YELLOW}Lưu ý: Biên dịch chéo sang macOS từ hệ điều hành khác cần có SDK macOS và công cụ osxcross.${NC}"
    fi
    exit 1
fi
