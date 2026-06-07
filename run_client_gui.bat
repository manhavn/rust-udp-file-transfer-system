@echo off
:: Script khởi chạy nhanh ứng dụng Desktop GUI (Tauri) trên Windows
chcp 65001 > nul

cd %~dp0client_gui

echo ==========================================================
:: Kiểm tra Node.js & npm
where node >nul 2>nul
if %errorlevel% neq 0 (
    echo Lỗi: Node.js chưa được cài đặt. Vui lòng cài đặt Node.js trước.
    pause
    exit /b 1
)

where npm >nul 2>nul
if %errorlevel% neq 0 (
    echo Lỗi: npm chưa được cài đặt.
    pause
    exit /b 1
)

echo 1. Đang kiểm tra thư viện dependencies của Node...
if not exist "node_modules" (
    echo Không tìm thấy thư mục node_modules. Đang chạy 'npm install'...
    call npm install
) else (
    echo    - Các thư viện Node.js đã sẵn sàng.
)

echo 2. Đang khởi chạy ứng dụng Desktop GUI...
echo Lưu ý: Hãy chắc chắn bạn đã cài đặt C++ Build Tools qua Visual Studio.
echo Xem hướng dẫn chi tiết trong tệp client_gui\README.md.
echo ==========================================================

call npm run tauri dev
