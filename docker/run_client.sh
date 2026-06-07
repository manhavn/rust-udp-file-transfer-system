#!/bin/bash
# Script khởi chạy Client bằng Docker (nạp tự động từ file .tar và mount file upload)

if [ -z "$1" ] || [ "$1" == "--help" ] || [ "$1" == "-h" ]; then
    echo "Sử dụng: $0 <đường_dẫn_file> [các tham số khác...]"
    echo ""
    echo "Các tham số cấu hình hỗ trợ:"
    echo "  --server-ip <ip>      Địa chỉ IP của Server (mặc định: 127.0.0.1)"
    echo "  --udp-port <port>     Cổng UDP của Server (mặc định: 5000)"
    echo "  --http-port <port>    Cổng HTTP của Server (mặc định: 8080)"
    echo "  --block-size <bytes>  Kích thước khối dữ liệu UDP gửi đi (mặc định: 16384)"
    echo "  --log-progress        Hiển thị tiến trình upload dạng log dòng mới"
    echo "  --password <password> Mật khẩu bảo mật tải xuống file (mặc định: không có)"
    echo ""
    echo "Ví dụ gửi file không mật khẩu:"
    echo "  $0 video.mp4 --server-ip 192.168.1.100"
    echo ""
    echo "Ví dụ gửi file có mật khẩu:"
    echo "  $0 video.mp4 --server-ip 192.168.1.100 --password mysecret123"
    exit 1
fi

FILE_PATH=$1
shift # Dịch chuyển để "$@" chỉ còn các flags tiếp theo

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
if ! docker image inspect rtk.udp/client >/dev/null 2>&1; then
    if [ -n "$TAR_FILE" ]; then
        echo "==> Đang nạp Client Image từ tệp cache ${TAR_FILE}..."
        docker load -i "$TAR_FILE"
    else
        echo "Lỗi: Không tìm thấy file thực thi 'rtk-udp-client.tar'."
        echo "Vui lòng chạy './build_container.sh' trước để biên dịch."
        exit 1
    fi
fi

echo "=========================================================="
echo "Khởi chạy RTK UDP Client qua Docker..."
echo "Tệp tin mount: $ABS_FILE_PATH"
echo "=========================================================="

# Mount thư mục chứa file và thực thi
docker run --rm -it \
  --add-host=host.docker.internal:host-gateway \
  -e SERVER_IP=host.docker.internal \
  -v "$DIR_PATH:/data:ro" \
  rtk.udp/client "/data/$FILE_NAME" "$@"
