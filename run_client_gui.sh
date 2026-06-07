#!/bin/bash
# Script để khởi chạy nhanh ứng dụng Desktop GUI (Tauri)

set -e

# Chỉ kiểm tra và cài đặt dependencies trên Linux
if [ "$(uname)" == "Linux" ]; then
    echo "=========================================================="
    echo "1. Đang kiểm tra các thư viện đồ họa hệ thống (GTK, WebKit, SSL)..."
    MISSING_DEPS=()
    
    # Kiểm tra sự tồn tại của pkg-config
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
        # Nếu không có pkg-config, giả định là cần cài đặt toàn bộ để đảm bảo an toàn
        MISSING_DEPS+=("libgtk-3-dev" "libwebkit2gtk-4.1-dev" "libssl-dev")
    fi
    
    if [ ${#MISSING_DEPS[@]} -ne 0 ]; then
        echo "   -> Phát hiện thiếu thư viện hệ thống: ${MISSING_DEPS[*]}"
        echo "   -> Đang chạy 'sudo apt-get update' và cài đặt các thư viện hệ thống..."
        echo "   (Hệ thống có thể yêu cầu bạn nhập mật khẩu sudo của máy mình)"
        echo "----------------------------------------------------------"
        sudo apt-get update
        sudo apt-get install -y "${MISSING_DEPS[@]}" build-essential libayatana-appindicator3-dev librsvg2-dev
        echo "----------------------------------------------------------"
        echo "   -> Đã hoàn thành cài đặt thư viện hệ thống."
    else
        echo "   -> Các thư viện hệ thống (GTK, WebKit, SSL) đã sẵn sàng."
    fi
fi

# Di chuyển tới thư mục client_gui
cd "$(dirname "$0")/client_gui"

echo "=========================================================="
# Kiểm tra sự tồn tại của Node.js & npm
if ! command -v node &> /dev/null; then
    echo "Lỗi: Node.js chưa được cài đặt. Vui lòng cài đặt Node.js trước."
    exit 1
fi

if ! command -v npm &> /dev/null; then
    echo "Lỗi: npm chưa được cài đặt."
    exit 1
fi

echo "2. Đang kiểm tra thư viện dependencies của Node..."
if [ ! -d "node_modules" ]; then
    echo "Không tìm thấy thư mục node_modules. Đang chạy 'npm install'..."
    npm install
else
    echo "   -> Các thư viện Node.js đã sẵn sàng."
fi

echo "3. Đang khởi chạy ứng dụng Desktop GUI..."
echo "Lưu ý: Ứng dụng Tauri sẽ được chạy dưới chế độ phát triển (Development mode)."
echo "=========================================================="

npm run tauri dev
