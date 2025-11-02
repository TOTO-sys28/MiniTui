@echo off
title MiniTUI Installer

:: ==============================
:: Elevate to Administrator
:: ==============================
net session >nul 2>&1
if %errorlevel% neq 0 (
echo Requesting administrator privileges...
powershell -Command "Start-Process '%~f0' -Verb RunAs"
exit /b
)

echo ============================================
echo Installing MiniTUI Music Player...
echo ============================================
echo.

:: ==============================
:: Create installation directory
:: ==============================
set "TARGET=%ProgramFiles%\MiniTUI"
if not exist "%TARGET%" (
echo Creating installation directory...
mkdir "%TARGET%"
)

:: ==============================
:: Copy executable
:: ==============================
echo Copying MiniTUI executable...
copy "%~dp0minitui.exe" "%TARGET%\minitui.exe" >nul

if not exist "%TARGET%\minitui.exe" (
echo ❌ Copy failed!
echo Make sure "minitui.exe" is in the same folder as this installer.
pause
exit /b
)
echo ✅ Copy successful.
echo.

:: ==============================
:: Add to system PATH safely
:: ==============================
set "NEWPATH=%TARGET%"
for /f "tokens=2*" %%a in ('reg query "HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment" /v Path 2^>nul') do set "CURPATH=%%b"

echo Updating system PATH...
echo %CURPATH% | find /i "%NEWPATH%" >nul
if errorlevel 1 (
reg add "HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment" /v Path /t REG_EXPAND_SZ /d "%CURPATH%;%NEWPATH%" /f >nul
echo ✅ Added MiniTUI to system PATH.
) else (
echo ⚠️ MiniTUI is already in the system PATH.
)

:: ==============================
:: Finish up
:: ==============================
echo.
echo ============================================
echo ✅ Installation complete!
echo ============================================
echo.
echo You can now run MiniTUI from any new
echo Command Prompt or PowerShell window by typing:
echo.
echo minitui
echo.
echo If it doesn’t start right away, open a new terminal.
echo.
pause
exit /b