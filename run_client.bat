@echo off
:: Script khởi chạy Client CLI ở môi trường Production trên Windows
chcp 65001 > nul

set BINARY=""
if exist "target\release\client_cli.exe" (
    set BINARY="target\release\client_cli.exe"
) else if exist "client_cli.exe" (
    set BINARY="client_cli.exe"
) else (
    echo Lỗi: Không tìm thấy file thực thi 'client_cli.exe'.
    echo Vui lòng chạy build client trước.
    exit /b 1
)

if "%~1"=="" (
    echo Sử dụng: %0 ^<đường_dẫn_file^> [ip_server] [cổng_udp] [cổng_http]
    echo Ví dụ: %0 movie.mp4 127.0.0.1 5000 8080
    exit /b 1
)

set FILE_PATH=%1
set SERVER_IP=%2
if "%2" == "" set SERVER_IP=127.0.0.1
set UDP_PORT=%3
if "%3" == "" set UDP_PORT=5000
set HTTP_PORT=%4
if "%4" == "" set HTTP_PORT=8080

if not exist "%FILE_PATH%" (
    echo Lỗi: File "%FILE_PATH%" không tồn tại.
    exit /b 1
)

echo ==========================================================
echo Khởi chạy RTK UDP Client trên Windows...
echo File cần gửi: %FILE_PATH%
echo Địa chỉ Server: %SERVER_IP%
echo Cổng UDP: %UDP_PORT% ^| Cổng HTTP: %HTTP_PORT%
echo ==========================================================

%BINARY% "%FILE_PATH%" --server-ip "%SERVER_IP%" --udp-port "%UDP_PORT%" --http-port "%HTTP_PORT%" --log-progress
