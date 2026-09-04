#!/bin/bash
set -e
echo "CloudSH Agent - Linux"
[ -z "$CLOUDSH_SERVER" ] && CLOUDSH_SERVER=https://cloudsh.yxpil.com

if [ -z "$CLOUDSH_CLIENT_ID" ] || [ -z "$CLOUDSH_PASSWORD" ]; then
  echo "Auto-registering..."
  RESP=$(curl -s -X POST $CLOUDSH_SERVER/register)
  export CLOUDSH_CLIENT_ID=$(awk -F'"' '/client_id/{print $4}' <<< "$RESP")
  export CLOUDSH_PASSWORD=$(awk -F'"' '/password/{print $4}' <<< "$RESP")
  echo "  client_id: $CLOUDSH_CLIENT_ID"
  echo "  password:  $CLOUDSH_PASSWORD"
fi

BIN=/usr/local/bin/cloudsh-agent

if curl -fsSL https://cloudsh.yxpil.com/bin/cloudsh-agent-linux-x86_64 -o "$BIN" 2>/dev/null; then
  echo "Downloaded prebuilt binary"
else
  if [ -f /etc/NIXOS ] && ! command -v cargo &>/dev/null; then
    echo "NixOS: run nix-shell -p cargo first, then re-run this script."
    exit 1
  fi
  echo "Compiling from source..."
  command -v cargo &>/dev/null || { curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; source "$HOME/.cargo/env"; }
  TMP=$(mktemp -d); cd "$TMP"
  curl -fsSL https://raw.githubusercontent.com/yxpil/CloudSH/main/Agent/Cargo.toml -o Cargo.toml
  mkdir -p Src
  curl -fsSL https://raw.githubusercontent.com/yxpil/CloudSH/main/Agent/Src/main.rs -o Src/main.rs
  cargo build --release -q
  sudo cp target/release/cloudsh-agent "$BIN"
fi

sudo chmod +x "$BIN"

sudo tee /etc/systemd/system/cloudsh-agent.service >/dev/null <<SVC
[Unit]
Description=CloudSH Agent
After=network.target
[Service]
Type=simple
ExecStart=$BIN
Environment=CLOUDSH_SERVER=$CLOUDSH_SERVER
Environment=CLOUDSH_CLIENT_ID=$CLOUDSH_CLIENT_ID
Environment=CLOUDSH_PASSWORD=$CLOUDSH_PASSWORD
Restart=always
RestartSec=5
[Install]
WantedBy=multi-user.target
SVC
sudo systemctl daemon-reload && sudo systemctl enable --now cloudsh-agent
echo "Agent started."
