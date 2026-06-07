@echo off
chcp 65001 > nul

if "%~1" == "" (
    echo Sử dụng: %~0 ^<đường_dẫn_file^> [ip_server] [cổng_udp] [cổng_http]
    echo Ví dụ: %~0 video.mp4 127.0.0.1 5000 8080
    pause
    exit /b 1
)

set FILE=%~1
set IP=%~2
if "%IP%"=="" set IP=127.0.0.1
set UDP_PORT=%~3
if "%UDP_PORT%"=="" set UDP_PORT=5000
set HTTP_PORT=%~4
if "%HTTP_PORT%"=="" set HTTP_PORT=8080

set BINARY=
if exist "target\x86_64-pc-windows-gnu\release\client_cli.exe" (
    set BINARY=target\x86_64-pc-windows-gnu\release\client_cli.exe
) else if exist "..\target\x86_64-pc-windows-gnu\release\client_cli.exe" (
    set BINARY=..\target\x86_64-pc-windows-gnu\release\client_cli.exe
) else if exist "target\x86_64-pc-windows-msvc\release\client_cli.exe" (
    set BINARY=target\x86_64-pc-windows-msvc\release\client_cli.exe
) else if exist "..\target\x86_64-pc-windows-msvc\release\client_cli.exe" (
    set BINARY=..\target\x86_64-pc-windows-msvc\release\client_cli.exe
) else if exist "target\release\client_cli.exe" (
    set BINARY=target\release\client_cli.exe
) else if exist "..\target\release\client_cli.exe" (
    set BINARY=..\target\release\client_cli.exe
) else if exist "client_cli.exe" (
    set BINARY=client_cli.exe
)

if "%BINARY%"=="" (
    echo Lỗi: Không tìm thấy file thực thi 'client_cli.exe'. Vui lòng biên dịch trước.
    pause
    exit /b 1
)

echo ==> Đang bắt đầu gửi file qua Windows client...
%BINARY% "%FILE%" --server-ip "%IP%" --udp-port "%UDP_PORT%" --http-port "%HTTP_PORT%"
