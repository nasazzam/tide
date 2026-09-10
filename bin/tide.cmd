@echo off
pwsh -NoProfile -ExecutionPolicy Bypass -File "%~dp0tide.ps1" %*
exit /b %ERRORLEVEL%
