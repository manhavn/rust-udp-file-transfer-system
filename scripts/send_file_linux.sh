#!/bin/bash

if [ -z "$1" ]; then
    echo "Sử dụng: $0 <đường_dẫn_file> [ip_server] [cổng_udp] [cổng_http]"
    echo "Ví dụ: $0 video.mp4 127.0.0.1 5000 8080"
    exit 1
fi

FILE=$1
IP=${2:-"127.0.0.1"}
UDP_PORT=${3:-"5000"}
HTTP_PORT=${4:-"8080"}

# Tìm đường dẫn chạy thích hợp
if [ -f "./target/release/client_cli" ]; then
    BINARY="./target/release/client_cli"
elif [ -f "../target/release/client_cli" ]; then
    BINARY="../target/release/client_cli"
elif [ -f "./client_cli" ]; then
    BINARY="./client_cli"
else
    echo "Lỗi: Không tìm thấy file thực thi 'client_cli'. Vui lòng chạy 'cargo build --release' trước."
    exit 1
fi

echo "==> Đang bắt đầu gửi file qua Linux client..."
$BINARY "$FILE" --server-ip "$IP" --udp-port "$UDP_PORT" --http-port "$HTTP_PORT"
