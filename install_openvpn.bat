@echo off
title OpenVPN Silent Automated Installer
setlocal EnableDelayedExpansion

:: Check for Administrative Privileges
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo ========================================================
    echo   Requesting Administrator Privileges...
    echo ========================================================
    powershell -NoProfile -Command "Start-Process cmd.exe -ArgumentList '/c \"\"%~f0\"\"' -Verb RunAs"
    exit /b
)

cd /d "%~dp0"
echo ========================================================
echo   OpenVPN Silent Automated Installer
echo ========================================================
echo.
echo [1/3] Downloading official OpenVPN MSI package...

powershell -NoProfile -ExecutionPolicy Bypass -Command ^
    "$url = 'https://swupdate.openvpn.org/community/releases/OpenVPN-2.7.7-I001-amd64.msi';" ^
    "$dest = Join-Path $env:TEMP 'OpenVPN-Setup.msi';" ^
    "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13;" ^
    "Invoke-WebRequest -Uri $url -OutFile $dest -UseBasicParsing;" ^
    "Write-Host '[2/3] Installing OpenVPN and Wintun driver silently...';" ^
    "$proc = Start-Process msiexec.exe -ArgumentList '/i', ('\"' + $dest + '\"'), '/quiet', '/norestart' -PassThru -Wait;" ^
    "Remove-Item $dest -Force -ErrorAction SilentlyContinue;" ^
    "if ($proc.ExitCode -eq 0 -or $proc.ExitCode -eq 3010) {" ^
    "    Write-Host '[3/3] Installation completed successfully!' -ForegroundColor Green;" ^
    "} else {" ^
    "    Write-Host ('Installation exited with code: ' + $proc.ExitCode) -ForegroundColor Yellow;" ^
    "}"

echo.
echo ========================================================
echo   Installation finished! You can close this window.
echo ========================================================
timeout /t 5
