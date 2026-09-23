@echo off
title VPN Gate Desktop Launcher
cd /d "%~dp0"

echo ========================================================
echo   Launching VPN Gate Windows Client...
echo ========================================================

python app.py

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Application exited with error code %ERRORLEVEL%.
    pause
)
