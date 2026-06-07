#!/bin/bash
# Script để đóng gói và biên dịch ứng dụng Desktop GUI (Tauri) cho Production

set -e

# Màu sắc hiển thị
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== UDP Transfer System - Build Script cho Desktop GUI ===${NC}"

# 1. Chỉ kiểm tra và cài đặt dependencies trên Linux
if [ "$(uname)" == "Linux" ]; then
    echo -e "${BLUE}[1/4] Đang kiểm tra các thư viện đồ họa hệ thống (GTK, WebKit, SSL)...${NC}"
    MISSING_DEPS=()
    
    if command -v pkg-config &> /dev/null; then
        if ! pkg-config --exists gdk-3.0; then
            MISSING_DEPS+=("libgtk-3-dev")
        fi
        if ! pkg-config --exists webkit2gtk-4.1; then
            MISSING_DEPS+=("libwebkit2gtk-4.1-dev")
        fi
        if ! pkg-config --exists openssl; then
            MISSING_DEPS+=("libssl-dev")
        fi
    else
        MISSING_DEPS+=("libgtk-3-dev" "libwebkit2gtk-4.1-dev" "libssl-dev")
    fi
    
    if [ ${#MISSING_DEPS[@]} -ne 0 ]; then
        echo -e "${YELLOW}   -> Phát hiện thiếu thư viện hệ thống: ${MISSING_DEPS[*]}${NC}"
        echo -e "${YELLOW}   -> Yêu cầu quyền sudo để cập nhật hệ thống và tự động cài đặt...${NC}"
        sudo apt-get update
        sudo apt-get install -y "${MISSING_DEPS[@]}" build-essential libayatana-appindicator3-dev librsvg2-dev
    else
        echo -e "${GREEN}   -> Các thư viện hệ thống đã đầy đủ.${NC}"
    fi
fi

# Di chuyển tới thư mục client_gui
cd "$(dirname "$0")/client_gui"

# 2. Kiểm tra Node.js & npm
echo -e "${BLUE}[2/4] Đang kiểm tra Node.js & npm...${NC}"
if ! command -v node &> /dev/null; then
    echo -e "${RED}Lỗi: Node.js chưa được cài đặt. Vui lòng cài đặt Node.js trước khi build GUI.${NC}"
    exit 1
fi
if ! command -v npm &> /dev/null; then
    echo -e "${RED}Lỗi: npm chưa được cài đặt.${NC}"
    exit 1
fi
echo -e "${GREEN}   -> Node.js và npm đã sẵn sàng.${NC}"

# 3. Kiểm tra node_modules
echo -e "${BLUE}[3/4] Đang chuẩn bị các gói Node dependencies...${NC}"
if [ ! -d "node_modules" ]; then
    echo "Không tìm thấy thư mục node_modules. Đang chạy 'npm install'..."
    npm install
else
    echo -e "${GREEN}   -> Các thư viện Node.js đã sẵn sàng.${NC}"
fi

# 4. Chạy tauri build
echo -e "${BLUE}[4/4] Bắt đầu quá trình biên dịch và đóng gói Tauri GUI...${NC}"
echo "Quá trình này sẽ biên dịch mã nguồn Rust tối ưu hóa ở chế độ Release."
echo "----------------------------------------------------------"

npm run tauri build

echo "----------------------------------------------------------"
echo -e "${GREEN}✔ Quá trình đóng gói thành công!${NC}"
echo -e "Tệp tin cài đặt ứng dụng đã được xuất ra tại:"
echo -e "${YELLOW}client_gui/src-tauri/target/release/bundle/${NC}"
