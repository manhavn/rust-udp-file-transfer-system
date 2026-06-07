#!/bin/bash
# Script khởi chạy Client CLI ở môi trường Production (sử dụng file build release)

# Xác định đường dẫn file thực thi client_cli
BINARY=""
if [ -f "./target/release/client_cli" ]; then
    BINARY="./target/release/client_cli"
elif [ -f "./target/x86_64-unknown-linux-musl/release/client_cli" ]; then
    BINARY="./target/x86_64-unknown-linux-musl/release/client_cli"
elif [ -f "./target/aarch64-unknown-linux-musl/release/client_cli" ]; then
    BINARY="./target/aarch64-unknown-linux-musl/release/client_cli"
elif [ -f "./client_cli" ]; then
    BINARY="./client_cli"
else
    echo "Lỗi: Không tìm thấy file thực thi 'client_cli'."
    echo "Vui lòng chạy './build_client.sh' trước để biên dịch."
    exit 1
fi

# Kiểm tra đối số đầu vào tối thiểu
if [ -z "$1" ]; then
    echo "Sử dụng: $0 <đường_dẫn_file> [ip_server] [cổng_udp] [cổng_http] [mật_khẩu]"
    echo "Ví dụ: $0 movie.mp4 127.0.0.1 5000 8080 mysecret123"
    exit 1
fi

FILE_PATH=$1
SERVER_IP=${2:-"127.0.0.1"}
UDP_PORT=${3:-"5000"}
HTTP_PORT=${4:-"8080"}
PASSWORD=$5

if [ ! -f "$FILE_PATH" ]; then
    echo "Lỗi: File '$FILE_PATH' không tồn tại."
    exit 1
fi

echo "=========================================================="
echo "Khởi chạy RTK UDP Client..."
echo "File cần gửi: $FILE_PATH"
echo "Địa chỉ Server: $SERVER_IP"
echo "Cổng UDP: $UDP_PORT | Cổng HTTP: $HTTP_PORT"
if [ -n "$PASSWORD" ]; then
    echo "Mật khẩu tải xuống: *****"
fi
echo "=========================================================="

if [ -n "$PASSWORD" ]; then
    exec $BINARY "$FILE_PATH" --server-ip "$SERVER_IP" --udp-port "$UDP_PORT" --http-port "$HTTP_PORT" --password "$PASSWORD" --log-progress
else
    exec $BINARY "$FILE_PATH" --server-ip "$SERVER_IP" --udp-port "$UDP_PORT" --http-port "$HTTP_PORT" --log-progress
fi
