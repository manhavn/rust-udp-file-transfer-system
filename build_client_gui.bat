@echo off
:: Script đóng gói và biên dịch ứng dụng Desktop GUI (Tauri) cho Production trên Windows
chcp 65001 > nul

echo ==========================================================
echo === UDP Transfer System - Build Script cho Windows GUI ===
echo ==========================================================

cd %~dp0client_gui

:: 1. Kiểm tra Node.js & npm
echo [1/3] Đang kiểm tra Node.js ^& npm...
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
echo    - Node.js và npm đã sẵn sàng.

:: 2. Kiểm tra node_modules
echo [2/3] Đang chuẩn bị các gói Node dependencies...
if not exist "node_modules" (
    echo Không tìm thấy thư mục node_modules. Đang chạy 'npm install'...
    call npm install
) else (
    echo    - Các thư viện Node.js đã sẵn sàng.
)

:: 3. Chạy tauri build
echo [3/3] Bắt đầu quá trình biên dịch và đóng gói Tauri GUI...
echo Quá trình này sẽ biên dịch mã nguồn Rust tối ưu hóa ở chế độ Release.
echo ----------------------------------------------------------

call npm run tauri build

if %errorlevel% neq 0 (
    echo ----------------------------------------------------------
    echo Lỗi: Quá trình đóng gói thất bại.
    echo Vui lòng đảm bảo bạn đã cài đặt Microsoft C++ Build Tools qua Visual Studio.
    pause
    exit /b 1
)

echo ----------------------------------------------------------
echo ✔ Quá trình đóng gói thành công!
echo Tệp tin cài đặt ứng dụng (.msi / .exe) đã được xuất ra tại:
echo client_gui\src-tauri\target\release\bundle\
pause
