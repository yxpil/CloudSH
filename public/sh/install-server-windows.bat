@echo off
echo CloudSH Server - Windows
where node >nul 2>nul || (echo Install Node.js first: https://nodejs.org && exit /b 1)
mkdir "%USERPROFILE%\cloudsh-server" 2>nul
cd /d "%USERPROFILE%\cloudsh-server"
echo {"name":"cloudsh","dependencies":{"express":"^4.21.0"}} > package.json
call npm install --silent
echo Start: cd %USERPROFILE%\cloudsh-server ^&^& node server.js
