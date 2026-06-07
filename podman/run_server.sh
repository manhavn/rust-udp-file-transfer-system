#!/bin/bash
# Script khởi chạy Server bằng Podman (nạp tự động từ file .tar thành phẩm)

# Xác định vị trí file tar server
if [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    echo "Sử dụng: [Cấu hình ENV] $0"
    echo ""
    echo "Các biến môi trường cấu hình hỗ trợ trên máy Host:"
    echo "  UDP_PORT             Cổng UDP lắng nghe (mặc định: 5000)"
    echo "  HTTP_PORT            Cổng HTTP REST API & Dashboard (mặc định: 8080)"
    echo "  UPLOAD_DIR           Thư mục chứa các tệp tải lên (mặc định: ./uploads)"
    echo "  DB_PATH              Đường dẫn cơ sở dữ liệu SQLite (mặc định: ./db/data.sqlite)"
    echo "  CLEANUP_INTERVAL     Chu kỳ quét dọn dẹp tệp tin tính bằng phút (mặc định: 5)"
    echo "  INCOMPLETE_TIMEOUT   Thời gian lưu trữ tệp chưa xong tính bằng phút (mặc định: 60)"
    echo "  COMPLETED_TIMEOUT    Thời gian lưu trữ tệp đã xong tính bằng phút (mặc định: 15)"
    echo "  DISABLE_REQUEST_LOG  Tắt logs HTTP/UDP (true/false, mặc định: false)"
    echo ""
    echo "Ví dụ chạy tùy biến cổng:"
    echo "  UDP_PORT=5005 HTTP_PORT=8085 $0"
    echo ""
    echo "Ví dụ cấu hình đầy đủ biến môi trường (Full ENV Example):"
    echo "  UDP_PORT=5000 HTTP_PORT=8080 UPLOAD_DIR=./uploads DB_PATH=./db/data.sqlite CLEANUP_INTERVAL=5 INCOMPLETE_TIMEOUT=60 COMPLETED_TIMEOUT=15 DISABLE_REQUEST_LOG=false $0"
    exit 0
fi

TAR_FILE=""
if [ -f "./rtk-udp-server.tar" ]; then
    TAR_FILE="./rtk-udp-server.tar"
elif [ -f "../rtk-udp-server.tar" ]; then
    TAR_FILE="../rtk-udp-server.tar"
fi

# Tự động nạp image nếu chưa tồn tại trong Podman registry
if ! podman image inspect rtk.udp/server >/dev/null 2>&1; then
    if [ -n "$TAR_FILE" ]; then
        echo "==> Đang nạp Server Image từ tệp cache ${TAR_FILE}..."
        podman load -i "$TAR_FILE"
    else
        echo "Lỗi: Không tìm thấy file thực thi 'rtk-udp-server.tar'."
        echo "Vui lòng chạy './build_container.sh' trước để biên dịch."
        exit 1
    fi
fi

# Tạo thư mục lưu trữ cục bộ nếu chưa có
mkdir -p uploads db

echo "=========================================================="
echo "Khởi chạy RTK UDP Server qua Podman..."
echo "=========================================================="

HOST_UDP_PORT=${UDP_PORT:-5000}
HOST_HTTP_PORT=${HTTP_PORT:-8080}

# Chạy Server Container với nhãn bảo mật :Z cho SELinux hoàn toàn bằng biến môi trường
podman run -d \
  --name rtk-server \
  -p "$HOST_UDP_PORT:$HOST_UDP_PORT/udp" \
  -p "$HOST_HTTP_PORT:$HOST_HTTP_PORT/tcp" \
  -e UDP_PORT="$HOST_UDP_PORT" \
  -e HTTP_PORT="$HOST_HTTP_PORT" \
  ${UPLOAD_DIR:+-e UPLOAD_DIR="$UPLOAD_DIR"} \
  ${DB_PATH:+-e DB_PATH="$DB_PATH"} \
  ${CLEANUP_INTERVAL:+-e CLEANUP_INTERVAL="$CLEANUP_INTERVAL"} \
  ${INCOMPLETE_TIMEOUT:+-e INCOMPLETE_TIMEOUT="$INCOMPLETE_TIMEOUT"} \
  ${COMPLETED_TIMEOUT:+-e COMPLETED_TIMEOUT="$COMPLETED_TIMEOUT"} \
  ${DISABLE_REQUEST_LOG:+-e DISABLE_REQUEST_LOG="$DISABLE_REQUEST_LOG"} \
  -v "$(pwd)/uploads:/app/uploads:Z" \
  -v "$(pwd)/db:/app/db:Z" \
  rtk.udp/server
