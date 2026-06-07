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

# Thiết lập cấu hình mặc định (ưu tiên ENV hệ thống nếu có)
UPLOAD_DIR="${UPLOAD_DIR:-"./uploads"}"
DB_PATH="${DB_PATH:-"./db/data.sqlite"}"

# Phân tích tham số để lấy upload-dir và db-path nếu có truyền vào
args_list=("$@")
for ((i=0; i<${#args_list[@]}; i++)); do
    if [ "${args_list[i]}" = "--upload-dir" ]; then
        UPLOAD_DIR="${args_list[i+1]}"
    elif [ "${args_list[i]}" = "--db-path" ]; then
        DB_PATH="${args_list[i+1]}"
    fi
done

# Lấy thư mục chứa database
DB_DIR=$(dirname "$DB_PATH")

# Tự động tạo các thư mục cần thiết
mkdir -p "$UPLOAD_DIR" "$DB_DIR"

echo "=========================================================="
echo "Khởi chạy RTK UDP Server ở chế độ Production..."
echo "Thư mục upload: $UPLOAD_DIR"
echo "Cơ sở dữ liệu: $DB_PATH"
echo "=========================================================="

# Chạy server và chuyển tiếp toàn bộ tham số truyền vào, mặc định tắt HTTP request log
exec $BINARY --disable-request-log "$@"
