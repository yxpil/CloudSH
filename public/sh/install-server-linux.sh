#!/bin/bash
set -e
echo "CloudSH Server - Linux"
command -v node &>/dev/null || { curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -; sudo apt-get install -y nodejs; }
sudo mkdir -p /opt/cloudsh
cd /opt/cloudsh
echo '{"name":"cloudsh","dependencies":{"express":"^4.21.0"}}' > package.json
npm install --silent 2>/dev/null
sudo tee /etc/systemd/system/cloudsh-server.service >/dev/null <<'SVC'
[Unit]
Description=CloudSH Server
After=network.target
[Service]
Type=simple
WorkingDirectory=/opt/cloudsh
ExecStart=/usr/bin/node server.js
Restart=always
[Install]
WantedBy=multi-user.target
SVC
sudo systemctl daemon-reload && sudo systemctl enable --now cloudsh-server
echo "Done. http://$(hostname -I | awk '{print $1}'):3000"
