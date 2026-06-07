#!/bin/bash
# Script khởi chạy Client bằng Podman (nạp tự động từ file .tar và mount file upload)

if [ -z "$1" ] || [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    echo "Sử dụng: [Cấu hình ENV] $0 <đường_dẫn_file>"
    echo ""
    echo "Cấu hình bằng cách thiết lập biến môi trường trên máy Host:"
    echo "  SERVER_IP          Địa chỉ IP của Server (mặc định: host.containers.internal)"
    echo "  UDP_PORT           Cổng UDP của Server (mặc định: 5000)"
    echo "  HTTP_PORT          Cổng HTTP của Server (mặc định: 8080)"
    echo "  BLOCK_SIZE         Kích thước khối dữ liệu UDP gửi đi (mặc định: 16384)"
    echo "  LOG_PROGRESS       Hiển thị tiến trình upload dạng log (true/false, mặc định: false)"
    echo "  DOWNLOAD_PASSWORD  Mật khẩu bảo mật tải xuống file (mặc định: không có)"
    echo ""
    echo "Ví dụ gửi file không mật khẩu:"
    echo "  SERVER_IP=192.168.1.100 $0 video.mp4"
    echo ""
    echo "Ví dụ gửi file có mật khẩu:"
    echo "  SERVER_IP=192.168.1.100 DOWNLOAD_PASSWORD=mysecret123 $0 video.mp4"
    echo ""
    echo "Ví dụ cấu hình đầy đủ biến môi trường (Full ENV Example):"
    echo "  SERVER_IP=192.168.1.100 UDP_PORT=5000 HTTP_PORT=8080 BLOCK_SIZE=16384 LOG_PROGRESS=true DOWNLOAD_PASSWORD=mysecret123 $0 video.mp4"
    exit 1
fi

FILE_PATH=$1

if [ ! -f "$FILE_PATH" ]; then
    echo "Lỗi: Tệp tin '$FILE_PATH' không tồn tại."
    exit 1
fi

# Chuyển đổi đường dẫn tương đối thành tuyệt đối để mount volume chính xác
ABS_FILE_PATH=$(realpath "$FILE_PATH")
DIR_PATH=$(dirname "$ABS_FILE_PATH")
FILE_NAME=$(basename "$ABS_FILE_PATH")

# Xác định vị trí file tar client
TAR_FILE=""
if [ -f "./rtk-udp-client.tar" ]; then
    TAR_FILE="./rtk-udp-client.tar"
elif [ -f "../rtk-udp-client.tar" ]; then
    TAR_FILE="../rtk-udp-client.tar"
fi

# Tự động nạp image nếu chưa có
if ! podman image inspect rtk.udp/client >/dev/null 2>&1; then
    if [ -n "$TAR_FILE" ]; then
        echo "==> Đang nạp Client Image từ tệp cache ${TAR_FILE}..."
        podman load -i "$TAR_FILE"
    else
        echo "Lỗi: Không tìm thấy file thực thi 'rtk-udp-client.tar'."
        echo "Vui lòng chạy './build_container.sh' trước để biên dịch."
        exit 1
    fi
fi

echo "=========================================================="
echo "Khởi chạy RTK UDP Client qua Podman..."
echo "Tệp tin mount: $ABS_FILE_PATH"
echo "=========================================================="

PODMAN_SERVER_IP=${SERVER_IP:-"host.containers.internal"}

# Mount thư mục chứa file và thực thi với nhãn bảo mật :ro,Z cho SELinux
podman run --rm -it \
  --add-host=host.containers.internal:host-gateway \
  -e FILE_PATH="/data/$FILE_NAME" \
  -e SERVER_IP="$PODMAN_SERVER_IP" \
  ${UDP_PORT:+-e UDP_PORT="$UDP_PORT"} \
  ${HTTP_PORT:+-e HTTP_PORT="$HTTP_PORT"} \
  ${BLOCK_SIZE:+-e BLOCK_SIZE="$BLOCK_SIZE"} \
  ${LOG_PROGRESS:+-e LOG_PROGRESS="$LOG_PROGRESS"} \
  ${DOWNLOAD_PASSWORD:+-e DOWNLOAD_PASSWORD="$DOWNLOAD_PASSWORD"} \
  -v "$DIR_PATH:/data:ro,Z" \
  rtk.udp/client
