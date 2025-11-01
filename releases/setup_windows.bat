@echo off
echo Installing MiniTUI Music Player...
echo.

REM Create installation directory
if not exist "%ProgramFiles%\MiniTUI" mkdir "%ProgramFiles%\MiniTUI"

REM Copy executable
copy "minitui.exe" "%ProgramFiles%\MiniTUI\minitui.exe"

REM Add to PATH (requires admin privileges)
setx PATH "%PATH%;%ProgramFiles%\MiniTUI" /M

echo.
echo Installation complete!
echo.
echo You can now run 'minitui' from any command prompt or PowerShell window.
echo.
echo To start using it:
echo   minitui
echo.
pause