#!/bin/bash
# Script build container (Docker/Podman) cho Server và Client CLI với cơ chế cache base images (.tar) và tự động dọn dẹp active registry

# Màu sắc hiển thị
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 1. Phát hiện Command Engine (Podman hoặc Docker)
ENGINE=""
if command -v podman >/dev/null 2>&1; then
    ENGINE="podman"
elif command -v docker >/dev/null 2>&1; then
    ENGINE="docker"
else
    echo -e "${RED}Lỗi: Không tìm thấy Docker hoặc Podman trên hệ thống!${NC}"
    exit 1
fi

echo -e "${BLUE}=== UDP Transfer System - Khởi tạo build Container sử dụng [${ENGINE}] ===${NC}\n"

# Tên file cache base
BUILDER_TAR=".rtk-builder-base.tar"
RUNTIME_TAR=".rtk-runtime-base.tar"
APP_DEP_TAR=".rtk-dep-cache.tar"

# Tên Image tags
BUILDER_TAG="rtk.builder/base:latest"
RUNTIME_TAG="rtk.runtime/base:latest"
APP_DEP_TAG="rtk.app/dep-cache:latest"

ensure_base_images() {
    # Xử lý Builder Base Image
    echo -e "${BLUE}→ Kiểm tra Builder Base Image (${BUILDER_TAG})...${NC}"
    if ! $ENGINE image inspect $BUILDER_TAG >/dev/null 2>&1; then
        if [ -f "$BUILDER_TAR" ]; then
            echo -e "${YELLOW}→ Tìm thấy tệp cache ${BUILDER_TAR}, đang tiến hành nạp...${NC}"
            $ENGINE load -i $BUILDER_TAR
        fi

        if ! $ENGINE image inspect $BUILDER_TAG >/dev/null 2>&1; then
            echo -e "${YELLOW}→ Không tìm thấy cache, tiến hành biên dịch Builder Base từ builder.Dockerfile...${NC}"
            $ENGINE build -f builder.Dockerfile -t $BUILDER_TAG .
            if [ $? -eq 0 ]; then
                echo -e "${GREEN}✔ Tạo thành công ${BUILDER_TAG}. Đang xuất tệp cache...${NC}"
                $ENGINE save $BUILDER_TAG -o $BUILDER_TAR
            else
                echo -e "${RED}Lỗi: Không thể biên dịch Builder Base!${NC}"
                exit 1
            fi
        fi
    fi

    # Xử lý Runtime Base Image
    echo -e "${BLUE}→ Kiểm tra Runtime Base Image (${RUNTIME_TAG})...${NC}"
    if ! $ENGINE image inspect $RUNTIME_TAG >/dev/null 2>&1; then
        if [ -f "$RUNTIME_TAR" ]; then
            echo -e "${YELLOW}→ Tìm thấy tệp cache ${RUNTIME_TAR}, đang tiến hành nạp...${NC}"
            $ENGINE load -i $RUNTIME_TAR
        fi

        if ! $ENGINE image inspect $RUNTIME_TAG >/dev/null 2>&1; then
            echo -e "${YELLOW}→ Không tìm thấy cache, tiến hành biên dịch Runtime Base từ runtime.Dockerfile...${NC}"
            $ENGINE build -f runtime.Dockerfile -t $RUNTIME_TAG .
            if [ $? -eq 0 ]; then
                echo -e "${GREEN}✔ Tạo thành công ${RUNTIME_TAG}. Đang xuất tệp cache...${NC}"
                $ENGINE save $RUNTIME_TAG -o $RUNTIME_TAR
            else
                echo -e "${RED}Lỗi: Không thể biên dịch Runtime Base!${NC}"
                exit 1
            fi
        fi
    fi
}

ensure_dep_cache() {
    # Xử lý App Dependency Cache Image
    echo -e "${BLUE}→ Kiểm tra App Dependency Cache Image (${APP_DEP_TAG})...${NC}"
    if ! $ENGINE image inspect $APP_DEP_TAG >/dev/null 2>&1; then
        if [ -f "$APP_DEP_TAR" ]; then
            echo -e "${YELLOW}→ Tìm thấy tệp cache ${APP_DEP_TAR}, đang tiến hành nạp...${NC}"
            $ENGINE load -i $APP_DEP_TAR
        fi

        if ! $ENGINE image inspect $APP_DEP_TAG >/dev/null 2>&1; then
            echo -e "${YELLOW}→ Không tìm thấy cache, tiến hành biên dịch App Dependency Cache từ dep-cache.Dockerfile...${NC}"
            $ENGINE build -f dep-cache.Dockerfile -t $APP_DEP_TAG .
            if [ $? -eq 0 ]; then
                echo -e "${GREEN}✔ Tạo thành công ${APP_DEP_TAG}. Đang xuất tệp cache...${NC}"
                $ENGINE save $APP_DEP_TAG -o $APP_DEP_TAR
            else
                echo -e "${RED}Lỗi: Không thể biên dịch App Dependency Cache!${NC}"
                exit 1
            fi
        fi
    fi
}

build_server() {
    ensure_base_images
    ensure_dep_cache

    echo -e "${BLUE}→ 1. Đang xây dựng builder stage để cập nhật dependencies và code mới...${NC}"
    $ENGINE build --target builder -f server.Dockerfile -t $APP_DEP_TAG .
    if [ $? -ne 0 ]; then
        echo -e "${RED}✘ Lỗi khi rebuild builder stage!${NC}"
        exit 1
    fi

    echo -e "${BLUE}→ 2. Lưu lại compile target và dependencies vào cache tar...${NC}"
    $ENGINE save $APP_DEP_TAG -o $APP_DEP_TAR

    echo -e "${BLUE}→ 3. Xây dựng Server Image (rtk.udp/server)...${NC}"
    $ENGINE build -f server.Dockerfile -t rtk.udp/server .
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✔ Build rtk.udp/server thành công!${NC}"
        echo -e "${BLUE}→ 4. Đang xuất thành phẩm ra tệp rtk-udp-server.tar...${NC}"
        $ENGINE save rtk.udp/server -o rtk-udp-server.tar
        echo -e "${GREEN}✔ Đã lưu rtk-udp-server.tar thành công.${NC}"
    else
        echo -e "${RED}✘ Build rtk.udp/server thất bại!${NC}"
        exit 1
    fi
}

build_client() {
    ensure_base_images
    ensure_dep_cache

    echo -e "${BLUE}→ 1. Đang xây dựng builder stage để cập nhật dependencies và code mới...${NC}"
    $ENGINE build --target builder -f client.Dockerfile -t $APP_DEP_TAG .
    if [ $? -ne 0 ]; then
        echo -e "${RED}✘ Lỗi khi rebuild builder stage!${NC}"
        exit 1
    fi

    echo -e "${BLUE}→ 2. Lưu lại compile target và dependencies vào cache tar...${NC}"
    $ENGINE save $APP_DEP_TAG -o $APP_DEP_TAR

    echo -e "${BLUE}→ 3. Xây dựng Client Image (rtk.udp/client)...${NC}"
    $ENGINE build -f client.Dockerfile -t rtk.udp/client .
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✔ Build rtk.udp/client thành công!${NC}"
        echo -e "${BLUE}→ 4. Đang xuất thành phẩm ra tệp rtk-udp-client.tar...${NC}"
        $ENGINE save rtk.udp/client -o rtk-udp-client.tar
        echo -e "${GREEN}✔ Đã lưu rtk-udp-client.tar thành công.${NC}"
    else
        echo -e "${RED}✘ Build rtk.udp/client thất bại!${NC}"
        exit 1
    fi
}

cleanup_all() {
    echo -e "\n${YELLOW}→ Đang dọn dẹp toàn bộ các images khỏi Docker/Podman Engine để tiết kiệm dung lượng...${NC}"
    $ENGINE rmi rtk.udp/server rtk.udp/client rtk.app/dep-cache:latest rtk.builder/base:latest rtk.runtime/base:latest >/dev/null 2>&1
    echo -e "${GREEN}✔ Đã xóa toàn bộ images trong container registry. Các sản phẩm đã được lưu trữ an toàn trong các file .tar!${NC}"
}

# Hiển thị Menu lựa chọn build thành phẩm
echo -e "${BLUE}Biên dịch ứng dụng đích:${NC}"
echo "1) Build Server Image (rtk.udp/server)"
echo "2) Build Client Image (rtk.udp/client)"
echo "3) Build Cả hai (Both)"
echo "4) Thoát"
read -p "Chọn dịch vụ muốn build (1-4): " choice
echo ""

case $choice in
    1)
        build_server
        cleanup_all
        ;;
    2)
        build_client
        cleanup_all
        ;;
    3)
        build_server
        build_client
        cleanup_all
        ;;
    *)
        echo -e "${YELLOW}Đã thoát.${NC}"
        exit 0
        ;;
esac

echo -e "\n${GREEN}✔ Hoàn tất toàn bộ quy trình!${NC}"
