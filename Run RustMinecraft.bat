@echo off
setlocal
cd /d "%~dp0"

set "BIN=%~dp0target\debug\rmc-client.exe"
set "CARGO=%USERPROFILE%\.cargo\bin\cargo.exe"

if exist "%BIN%" (
    "%BIN%" play %*
    if errorlevel 1 pause
    exit /b %errorlevel%
)

if exist "%CARGO%" (
    "%CARGO%" run -p rmc-client -- play %*
    if errorlevel 1 pause
    exit /b %errorlevel%
)

echo rmc-client.exe and cargo.exe were not found.
echo Build the workspace once or install Rust at %%USERPROFILE%%\.cargo\bin\cargo.exe.
pause
exit /b 1
