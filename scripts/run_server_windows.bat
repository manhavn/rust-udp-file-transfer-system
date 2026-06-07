@echo off
:: Cấu hình UTF-8 để hiển thị tiếng Việt chính xác trên Windows Command Prompt
chcp 65001 > nul

set BINARY=
if exist "target\x86_64-pc-windows-gnu\release\server.exe" (
    set BINARY=target\x86_64-pc-windows-gnu\release\server.exe
) else if exist "..\target\x86_64-pc-windows-gnu\release\server.exe" (
    set BINARY=..\target\x86_64-pc-windows-gnu\release\server.exe
) else if exist "target\x86_64-pc-windows-msvc\release\server.exe" (
    set BINARY=target\x86_64-pc-windows-msvc\release\server.exe
) else if exist "..\target\x86_64-pc-windows-msvc\release\server.exe" (
    set BINARY=..\target\x86_64-pc-windows-msvc\release\server.exe
) else if exist "target\release\server.exe" (
    set BINARY=target\release\server.exe
) else if exist "..\target\release\server.exe" (
    set BINARY=..\target\release\server.exe
) else if exist "server.exe" (
    set BINARY=server.exe
)

if "%BINARY%"=="" (
    echo Lỗi: Không tìm thấy file thực thi 'server.exe'. Vui lòng biên dịch trước.
    pause
    exit /b 1
)

echo ==> Đang khởi chạy UDP ^& HTTP Server trên Windows...
%BINARY%
