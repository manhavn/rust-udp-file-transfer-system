#!/bin/bash

# Tìm đường dẫn chạy thích hợp
if [ -f "./target/release/server" ]; then
    BINARY="./target/release/server"
elif [ -f "../target/release/server" ]; then
    BINARY="../target/release/server"
elif [ -f "./target/x86_64-unknown-linux-musl/release/server" ]; then
    BINARY="./target/x86_64-unknown-linux-musl/release/server"
elif [ -f "../target/x86_64-unknown-linux-musl/release/server" ]; then
    BINARY="../target/x86_64-unknown-linux-musl/release/server"
elif [ -f "./target/aarch64-unknown-linux-musl/release/server" ]; then
    BINARY="./target/aarch64-unknown-linux-musl/release/server"
elif [ -f "../target/aarch64-unknown-linux-musl/release/server" ]; then
    BINARY="../target/aarch64-unknown-linux-musl/release/server"
elif [ -f "./server" ]; then
    BINARY="./server"
else
    echo "Lỗi: Không tìm thấy file thực thi 'server'. Vui lòng chạy 'cargo build --release' trước."
    exit 1
fi

echo "==> Đang khởi chạy UDP & HTTP Server trên Linux..."
$BINARY "$@"
