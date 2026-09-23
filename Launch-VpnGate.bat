@echo off
title Launch VPN Gate .NET 8 Native Desktop
cd /d "%~dp0"

if exist "dist\VpnGate.Desktop.exe" (
    start "" "dist\VpnGate.Desktop.exe"
) else (
    echo Binary not found in dist. Building now...
    call "%~dp0build.bat"
    start "" "dist\VpnGate.Desktop.exe"
)
