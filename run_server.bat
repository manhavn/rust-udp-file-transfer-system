@echo off
:: Script khởi chạy Server ở môi trường Production trên Windows
chcp 65001 > nul

set BINARY=""
if exist "target\release\server.exe" (
    set BINARY="target\release\server.exe"
) else if exist "server.exe" (
    set BINARY="server.exe"
) else (
    echo Lỗi: Không tìm thấy file thực thi 'server.exe'.
    echo Vui lòng chạy build server trước.
    exit /b 1
)

:: Tự động tạo các thư mục cần thiết
if not exist "uploads" mkdir uploads
if not exist "db" mkdir db

echo ==========================================================
echo Khởi chạy RTK UDP Server ở chế độ Production trên Windows...
echo Thư mục upload: .\uploads
echo Cơ sở dữ liệu: .\db\data.sqlite
echo ==========================================================

%BINARY% --disable-request-log %*
