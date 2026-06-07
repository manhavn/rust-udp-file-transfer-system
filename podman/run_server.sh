#!/bin/bash
# Script khởi chạy Server bằng Podman (nạp tự động từ file .tar thành phẩm)

# Xác định vị trí file tar server
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

# Chạy Server Container với nhãn bảo mật :Z cho SELinux
podman run -d \
  --name rtk-server \
  -p 5000:5000/udp \
  -p 8080:8080/tcp \
  -v "$(pwd)/uploads:/app/uploads:Z" \
  -v "$(pwd)/db:/app/db:Z" \
  rtk.udp/server "$@"
