@echo off
title Build VPN Gate .NET 8 Native
cd /d "%~dp0"

echo ========================================================
echo   Building VPN Gate .NET 8 Desktop Application...
echo ========================================================

set "DOTNET_CMD=dotnet"
where dotnet >nul 2>&1
if %ERRORLEVEL% neq 0 (
    set "DOTNET_CMD=%LOCALAPPDATA%\Microsoft\dotnet\dotnet.exe"
)

"%DOTNET_CMD%" publish "src\VpnGate.Desktop\VpnGate.Desktop.csproj" -c Release -o "dist"

if %ERRORLEVEL% equ 0 (
    echo.
    echo ========================================================
    echo   Build Succeeded!
    echo   Binary output: %~dp0dist\VpnGate.Desktop.exe
    echo ========================================================
) else (
    echo.
    echo Build failed with error code %ERRORLEVEL%.
    pause
)
