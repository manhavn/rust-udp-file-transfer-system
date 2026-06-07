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

:: Thiết lập giá trị mặc định
set UPLOAD_DIR=./uploads
set DB_PATH=./db/data.sqlite

:: Phân tích đối số dòng lệnh để tìm --upload-dir và --db-path
setlocal enabledelayedexpansion
set "prev="
for %%a in (%*) do (
    if "!prev!"=="--upload-dir" (
        set "UPLOAD_DIR=%%~a"
    )
    if "!prev!"=="--db-path" (
        set "DB_PATH=%%~a"
    )
    set "prev=%%~a"
)

:: Trích xuất thư mục chứa db
for %%i in ("%DB_PATH%") do set "DB_DIR=%%~dpi"

:: Tự động tạo các thư mục cần thiết
if not exist "%UPLOAD_DIR%" mkdir "%UPLOAD_DIR%"
if not exist "%DB_DIR%" mkdir "%DB_DIR%"

echo ==========================================================
echo Khởi chạy RTK UDP Server ở chế độ Production trên Windows...
echo Thư mục upload: %UPLOAD_DIR%
echo Cơ sở dữ liệu: %DB_PATH%
echo ==========================================================

endlocal & %BINARY% --disable-request-log %*
