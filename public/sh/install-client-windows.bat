@echo off
echo CloudSH Client - Windows
set BIN=%USERPROFILE%\cloudsh.exe
curl -fsSL https://cloudsh.yxpil.com/bin/cloudsh-windows-x86_64.exe -o "%BIN%" 2>nul || (
  echo No prebuilt binary, compile in WSL
  exit /b 1
)
echo Done. %BIN% -s http://SERVER:3000 register
