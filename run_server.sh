#!/bin/bash
# Script khởi chạy Server ở môi trường Production (sử dụng file build release)

# Xác định đường dẫn file thực thi server
BINARY=""
if [ -f "./target/release/server" ]; then
    BINARY="./target/release/server"
elif [ -f "./server" ]; then
    BINARY="./server"
else
    echo "Lỗi: Không tìm thấy file thực thi 'server'."
    echo "Vui lòng chạy './build_server.sh' trước để biên dịch."
    exit 1
fi

# Tự động tạo các thư mục cần thiết cho production nếu chưa có
mkdir -p ./uploads ./db

echo "=========================================================="
echo "Khởi chạy RTK UDP Server ở chế độ Production..."
echo "Thư mục upload: ./uploads"
echo "Cơ sở dữ liệu: ./db/data.sqlite"
echo "=========================================================="

# Chạy server và chuyển tiếp toàn bộ tham số truyền vào
exec $BINARY "$@"
