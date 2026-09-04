#!/bin/bash
set -e
echo "CloudSH Server - macOS"
command -v node &>/dev/null || brew install node
mkdir -p ~/cloudsh-server
cd ~/cloudsh-server
echo '{"name":"cloudsh","dependencies":{"express":"^4.21.0"}}' > package.json
npm install --silent 2>/dev/null
cat > ~/Library/LaunchAgents/com.cloudsh.server.plist << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>com.cloudsh.server</string>
<key>ProgramArguments</key><array><string>/usr/local/bin/node</string><string>server.js</string></array>
<key>WorkingDirectory</key><string>$HOME/cloudsh-server</string>
<key>RunAtLoad</key><true/>
<key>KeepAlive</key><true/>
</dict></plist>
PLIST
launchctl load ~/Library/LaunchAgents/com.cloudsh.server.plist
echo "Done. http://localhost:3000"
