@echo off
echo CloudSH Agent - Windows
if "%CLOUDSH_SERVER%"=="" set CLOUDSH_SERVER=https://cloudsh.yxpil.com
if "%CLOUDSH_CLIENT_ID%"=="" echo Auto-register: curl -X POST https://cloudsh.yxpil.com/register
if "%CLOUDSH_PASSWORD%"=="" echo Auto-register: curl -X POST https://cloudsh.yxpil.com/register
set BIN=%USERPROFILE%\cloudsh-agent.exe
curl -fsSL https://cloudsh.yxpil.com/bin/cloudsh-agent-windows-x86_64.exe -o "%BIN%" 2>nul || (
  echo No prebuilt binary, compile in WSL
  exit /b 1
)
echo Start: %BIN%
