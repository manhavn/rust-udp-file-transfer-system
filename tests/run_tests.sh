#!/bin/bash
# Script chạy các test case lỗi để kiểm tra khả năng xử lý lỗi của hệ thống UDP Transfer

# Màu sắc hiển thị
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

TEST_DIR="$(dirname "$(realpath "$0")")"
PROJECT_ROOT="$(dirname "$TEST_DIR")"
TEST_FILES_DIR="$TEST_DIR/test_files"

mkdir -p "$TEST_FILES_DIR"

# Tạo các file test mock
touch "$TEST_FILES_DIR/empty.bin"
echo "Hello World! This is a dummy test file for error testing." > "$TEST_FILES_DIR/dummy.bin"
head -c 102400 /dev/zero > "$TEST_FILES_DIR/large.bin"

# Phát hiện client thực thi
CLIENT_CMD=""
IS_CONTAINER=0

if [ -f "$PROJECT_ROOT/target/release/client_cli" ]; then
    CLIENT_CMD="$PROJECT_ROOT/target/release/client_cli"
    echo -e "${GREEN}✔ Phát hiện client CLI bản Native tại: $CLIENT_CMD${NC}"
elif command -v podman >/dev/null 2>&1 && podman image inspect rtk.udp/client >/dev/null 2>&1; then
    CLIENT_CMD="podman run --rm -i --add-host=host.containers.internal:host-gateway -e SERVER_IP=host.containers.internal -v $TEST_FILES_DIR:/data:ro,Z rtk.udp/client"
    IS_CONTAINER=1
    echo -e "${GREEN}✔ Phát hiện client CLI bản Podman container${NC}"
elif command -v docker >/dev/null 2>&1 && docker image inspect rtk.udp/client >/dev/null 2>&1; then
    CLIENT_CMD="docker run --rm -i --add-host=host.docker.internal:host-gateway -e SERVER_IP=host.docker.internal -v $TEST_FILES_DIR:/data:ro rtk.udp/client"
    IS_CONTAINER=1
    echo -e "${GREEN}✔ Phát hiện client CLI bản Docker container${NC}"
else
    # Thử build native nhanh để test
    echo -e "${YELLOW}⚠ Không tìm thấy Client thực thi. Đang tự động build Client Native...${NC}"
    (cd "$PROJECT_ROOT" && cargo build --release --bin client_cli)
    if [ -f "$PROJECT_ROOT/target/release/client_cli" ]; then
        CLIENT_CMD="$PROJECT_ROOT/target/release/client_cli"
        echo -e "${GREEN}✔ Build thành công client CLI bản Native tại: $CLIENT_CMD${NC}"
    else
        echo -e "${RED}✘ Không thể tìm thấy hoặc build client CLI để chạy test!${NC}"
        exit 1
    fi
fi

# Hàm chạy client và kiểm tra mã trả về
run_test_case() {
    local title="$1"
    local args="$2"
    local expected_status="$3" # 0 = dự kiến thành công, 1 = dự kiến thất bại
    local description="$4"

    echo -e "\n${BLUE}======================================================================${NC}"
    echo -e "${BLUE}▶ TEST CASE: $title${NC}"
    echo -e "${YELLOW}Mô tả: $description${NC}"
    echo -e "${BLUE}======================================================================${NC}"

    # Chuẩn bị tham số lệnh
    local final_args=""
    if [ $IS_CONTAINER -eq 1 ]; then
        # Nếu chạy trong container, đường dẫn file cần trỏ tới mount point /data/
        for arg in $args; do
            if [[ "$arg" == "$TEST_FILES_DIR/"* ]]; then
                local filename=$(basename "$arg")
                final_args="$final_args /data/$filename"
            else
                final_args="$final_args $arg"
            fi
        done
    else
        final_args="$args"
    fi

    echo -e "Thực thi: $CLIENT_CMD $final_args"
    # Thực hiện chạy lệnh
    $CLIENT_CMD $final_args
    local exit_code=$?

    if [ $expected_status -eq 0 ]; then
        if [ $exit_code -eq 0 ]; then
            echo -e "${GREEN}✔ KẾT QUẢ: ĐÚNG MONG ĐỢI (Thành công - Exit Code $exit_code)${NC}"
        else
            echo -e "${RED}✘ KẾT QUẢ: SAI MONG ĐỢI (Gặp lỗi khi cần thành công - Exit Code $exit_code)${NC}"
        fi
    else
        if [ $exit_code -ne 0 ]; then
            echo -e "${GREEN}✔ KẾT QUẢ: ĐÚNG MONG ĐỢI (Chặn/báo lỗi thành công - Exit Code $exit_code)${NC}"
        else
            echo -e "${RED}✘ KẾT QUẢ: SAI MONG ĐỢI (Chạy thành công khi cần phải báo lỗi - Exit Code $exit_code)${NC}"
        fi
    fi
}

# ----------------- CHẠY CÁC TEST CASE -----------------

# Case 1: Đường dẫn file không tồn tại
run_test_case \
    "Đường dẫn File Không Tồn Tại" \
    "$TEST_FILES_DIR/this_file_does_not_exist.bin" \
    1 \
    "Kiểm tra xem client có tự phát hiện lỗi và dừng ngay lập tức khi file chỉ định không tồn tại trên ổ đĩa hay không."

# Case 2: File có độ dài 0 byte (Empty file)
# Bật server trước khi chạy để kiểm tra thành công, hoặc kiểm tra xem client có tự hoàn thành ngay không
run_test_case \
    "File Trống (0 Bytes)" \
    "$TEST_FILES_DIR/empty.bin" \
    0 \
    "Kiểm tra xử lý file trống. Giao thức không cần gửi block dữ liệu nào và phải hoàn tất thành công ngay lập tức."

# Case 3: Sai cổng UDP Server
run_test_case \
    "Sai Cổng UDP Server" \
    "$TEST_FILES_DIR/dummy.bin --udp-port 4999" \
    1 \
    "Kiểm tra timeout UDP. Gửi tới cổng UDP không hợp lệ (không có server lắng nghe). Client phải báo lỗi thất bại sau 30 lần thử lại."

# Case 4: Sai địa chỉ IP Server (Không tồn tại / Sai host)
run_test_case \
    "Sai Địa Chỉ IP Server" \
    "$TEST_FILES_DIR/dummy.bin --server-ip 192.0.2.1 --udp-port 5000" \
    1 \
    "Địa chỉ IP 192.0.2.1 là IP thử nghiệm (không thể kết nối). Lỗi kết nối HTTP API đăng ký sẽ xảy ra, client thử fallback trực tiếp qua UDP nhưng cũng sẽ thất bại và báo lỗi."

# Case 5: Kích thước block bằng 0
run_test_case \
    "Block Size Bằng 0" \
    "$TEST_FILES_DIR/dummy.bin --block-size 0" \
    1 \
    "Tham số block size bằng 0 không hợp lệ. Chương trình Client phải từ chối và báo lỗi ngay lập tức."

# Case 6: Kích thước block quá lớn (Vượt quá giới hạn truyền tải UDP)
run_test_case \
    "Block Size Quá Lớn (90KB)" \
    "$TEST_FILES_DIR/large.bin --block-size 90000" \
    1 \
    "UDP hỗ trợ kích thước tối đa là 65,535 bytes (bao gồm cả IP/UDP header). Cấu hình block size 90KB phải bị hệ thống báo lỗi không thể gửi gói tin hoặc lỗi truyền tải."

echo -e "\n${BLUE}======================================================================${NC}"
echo -e "${GREEN}✔ Hoàn tất toàn bộ quy trình chạy test case lỗi!${NC}"
echo -e "${BLUE}======================================================================${NC}"
