#!/bin/bash
# Script build container (Docker/Podman) cho Server và Client CLI với cơ chế cache base images (.tar)

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

# Tên Image tags
BUILDER_TAG="rtk-builder-base:latest"
RUNTIME_TAG="rtk-runtime-base:latest"

# 2. Xử lý Builder Base Image
echo -e "${BLUE}[1/4] Kiểm tra Builder Base Image (${BUILDER_TAG})...${NC}"
if $ENGINE image inspect $BUILDER_TAG >/dev/null 2>&1; then
    echo -e "${GREEN}✔ Builder Base Image đã tồn tại cục bộ.${NC}"
else
    if [ -f "$BUILDER_TAR" ]; then
        echo -e "${YELLOW}→ Tìm thấy tệp cache ${BUILDER_TAR}, đang tiến hành nạp...${NC}"
        $ENGINE load -i $BUILDER_TAR
    fi

    if $ENGINE image inspect $BUILDER_TAG >/dev/null 2>&1; then
        echo -e "${GREEN}✔ Nạp thành công Builder Base từ tệp cache.${NC}"
    else
        echo -e "${YELLOW}→ Không tìm thấy cache, tiến hành biên dịch Builder Base từ builder.Dockerfile...${NC}"
        $ENGINE build -f builder.Dockerfile -t $BUILDER_TAG .
        if [ $? -eq 0 ]; then
            echo -e "${GREEN}✔ Tạo thành công ${BUILDER_TAG}. Đang xuất tệp cache...${NC}"
            $ENGINE save $BUILDER_TAG -o $BUILDER_TAR
        else
            echo -e "${RED}✘ Lỗi khi biên dịch Builder Base!${NC}"
            exit 1
        fi
    fi
fi
echo ""

# 3. Xử lý Runtime Base Image
echo -e "${BLUE}[2/4] Kiểm tra Runtime Base Image (${RUNTIME_TAG})...${NC}"
if $ENGINE image inspect $RUNTIME_TAG >/dev/null 2>&1; then
    echo -e "${GREEN}✔ Runtime Base Image đã tồn tại cục bộ.${NC}"
else
    if [ -f "$RUNTIME_TAR" ]; then
        echo -e "${YELLOW}→ Tìm thấy tệp cache ${RUNTIME_TAR}, đang tiến hành nạp...${NC}"
        $ENGINE load -i $RUNTIME_TAR
    fi

    if $ENGINE image inspect $RUNTIME_TAG >/dev/null 2>&1; then
        echo -e "${GREEN}✔ Nạp thành công Runtime Base từ tệp cache.${NC}"
    else
        echo -e "${YELLOW}→ Không tìm thấy cache, tiến hành biên dịch Runtime Base từ runtime.Dockerfile...${NC}"
        $ENGINE build -f runtime.Dockerfile -t $RUNTIME_TAG .
        if [ $? -eq 0 ]; then
            echo -e "${GREEN}✔ Tạo thành công ${RUNTIME_TAG}. Đang xuất tệp cache...${NC}"
            $ENGINE save $RUNTIME_TAG -o $RUNTIME_TAR
        else
            echo -e "${RED}✘ Lỗi khi biên dịch Runtime Base!${NC}"
            exit 1
        fi
    fi
fi
echo ""

# 4. Hiển thị Menu lựa chọn build thành phẩm
echo -e "${BLUE}[3/4] Biên dịch ứng dụng đích:${NC}"
echo "1) Build Server Image (rtk-udp-server)"
echo "2) Build Client Image (rtk-udp-client)"
echo "3) Build Cả hai (Both)"
echo "4) Thoát"
read -p "Chọn dịch vụ muốn build (1-4): " choice
echo ""

build_server() {
    echo -e "${BLUE}→ Đang xây dựng Server Image (rtk-udp-server)...${NC}"
    $ENGINE build -f server.Dockerfile -t rtk-udp-server .
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✔ Build rtk-udp-server thành công!${NC}"
    else
        echo -e "${RED}✘ Build rtk-udp-server thất bại!${NC}"
        exit 1
    fi
}

build_client() {
    echo -e "${BLUE}→ Đang xây dựng Client Image (rtk-udp-client)...${NC}"
    $ENGINE build -f client.Dockerfile -t rtk-udp-client .
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✔ Build rtk-udp-client thành công!${NC}"
    else
        echo -e "${RED}✘ Build rtk-udp-client thất bại!${NC}"
        exit 1
    fi
}

case $choice in
    1)
        build_server
        ;;
    2)
        build_client
        ;;
    3)
        build_server
        build_client
        ;;
    *)
        echo -e "${YELLOW}Đã thoát.${NC}"
        exit 0
        ;;
esac

echo -e "\n${GREEN}✔ Hoàn tất toàn bộ quy trình!${NC}"
