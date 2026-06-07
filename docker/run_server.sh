#!/bin/bash
# Script khởi chạy Server bằng Docker (nạp tự động từ file .tar thành phẩm)

# Xác định vị trí file tar server
if [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    echo "Sử dụng: $0 [các tham số cấu hình...]"
    echo ""
    echo "Các tham số cấu hình hỗ trợ:"
    echo "  --udp-port <port>          Cổng UDP lắng nghe (mặc định: 5000)"
    echo "  --http-port <port>         Cổng HTTP REST API & Dashboard (mặc định: 8080)"
    echo "  --upload-dir <dir>         Thư mục chứa các tệp tải lên (mặc định: ./uploads)"
    echo "  --db-path <path>           Đường dẫn cơ sở dữ liệu SQLite (mặc định: ./db/data.sqlite)"
    echo "  --cleanup-interval <min>   Chu kỳ quét dọn dẹp tệp tin (mặc định: 5 phút)"
    echo "  --incomplete-timeout <min> Thời gian lưu trữ tệp chưa xong (mặc định: 60 phút)"
    echo "  --completed-timeout <min>  Thời gian lưu trữ tệp đã xong (mặc định: 15 phút)"
    echo "  --disable-request-log      Tắt logs HTTP/UDP"
    echo ""
    echo "Ví dụ chạy tùy biến cổng:"
    echo "  $0 --udp-port 5005 --http-port 8085"
    exit 0
fi

TAR_FILE=""
if [ -f "./rtk-udp-server.tar" ]; then
    TAR_FILE="./rtk-udp-server.tar"
elif [ -f "../rtk-udp-server.tar" ]; then
    TAR_FILE="../rtk-udp-server.tar"
fi

# Tự động nạp image nếu chưa tồn tại trong Docker registry
if ! docker image inspect rtk.udp/server >/dev/null 2>&1; then
    if [ -n "$TAR_FILE" ]; then
        echo "==> Đang nạp Server Image từ tệp cache ${TAR_FILE}..."
        docker load -i "$TAR_FILE"
    else
        echo "Lỗi: Không tìm thấy file thực thi 'rtk-udp-server.tar'."
        echo "Vui lòng chạy './build_container.sh' trước để biên dịch."
        exit 1
    fi
fi

# Tạo thư mục lưu trữ cục bộ nếu chưa có
mkdir -p uploads db

echo "=========================================================="
echo "Khởi chạy RTK UDP Server qua Docker..."
echo "=========================================================="

# Chạy Server Container
docker run -d \
  --name rtk-server \
  -p 5000:5000/udp \
  -p 8080:8080/tcp \
  -v "$(pwd)/uploads:/app/uploads" \
  -v "$(pwd)/db:/app/db" \
  rtk.udp/server "$@"
