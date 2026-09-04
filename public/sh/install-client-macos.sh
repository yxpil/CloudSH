#!/bin/bash
set -e
echo "CloudSH Client - macOS"
BIN=/usr/local/bin/cloudsh
if curl -fsSL https://cloudsh.yxpil.com/bin/cloudsh-darwin-arm64 -o "$BIN" 2>/dev/null; then
  echo "Downloaded prebuilt binary"
else
  echo "Compiling from source..."
  command -v cargo &>/dev/null || { curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; source "$HOME/.cargo/env"; }
  TMP=$(mktemp -d); cd "$TMP"
  curl -fsSL https://raw.githubusercontent.com/yxpil/CloudSH/main/Client/Cargo.toml -o Cargo.toml
  mkdir -p Src
  curl -fsSL https://raw.githubusercontent.com/yxpil/CloudSH/main/Client/Src/main.rs -o Src/main.rs
  cargo build --release -q
  sudo cp target/release/cloudsh "$BIN"
fi
sudo chmod +x "$BIN"
echo "Done. cloudsh -s http://SERVER:3000 register"
