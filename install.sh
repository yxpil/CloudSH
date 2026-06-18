#!/bin/bash
# CloudSH 一键安装
# curl -fsSL https://cloudsh.yxpil.com/install.sh | bash
# curl -fsSL https://cloudsh.yxpil.com/install.sh | bash -s server
set -e

TARGET="${1:-all}"
REPO="https://raw.githubusercontent.com/yxpil/CloudSH/main"
BIN_BASE="https://cloudsh.yxpil.com/bin"
BINDIR="${INSTALL_DIR:-/usr/local/bin}"
SERVER="${CLOUDSH_SERVER:-http://localhost:3000}"
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$ARCH" in x86_64|amd64) ARCH="x86_64";; aarch64|arm64) ARCH="arm64";; esac
PLAT="${OS}-${ARCH}"

TMPDIR=$(mktemp -d); trap 'rm -rf "$TMPDIR"' EXIT
GREEN='\033[0;32m'; CYAN='\033[0;36m'; NC='\033[0m'
echo -e "${CYAN}=== CloudSH Installer ===${NC}"

install_bin() {
    local name="$1" bin="$2"
    local url="${BIN_BASE}/${bin}-${PLAT}"
    echo -e "${GREEN}[下载] $name${NC}"
    if curl -fsSL "$url" -o "$TMPDIR/$bin" 2>/dev/null; then
        chmod +x "$TMPDIR/$bin"
        sudo mv "$TMPDIR/$bin" "$BINDIR/$bin"
        echo "  -> $BINDIR/$bin"
        return 0
    fi
    echo "  无预编译二进制，切换源码编译..."
    return 1
}

build_src() {
    local name="$1" dir="$2" bin="$3"
    if ! command -v cargo &>/dev/null; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
    echo -e "${GREEN}[编译] $name${NC}"
    mkdir -p "$TMPDIR/$dir/Src"
    curl -fsSL "$REPO/public/$dir/Cargo.toml" -o "$TMPDIR/$dir/Cargo.toml"
    curl -fsSL "$REPO/public/$dir/Src/main.rs" -o "$TMPDIR/$dir/Src/main.rs"
    cd "$TMPDIR/$dir"
    cargo build --release -q 2>&1 | tail -1
    sudo cp "target/release/$bin" "$BINDIR/$bin"
    sudo chmod +x "$BINDIR/$bin"
    echo "  -> $BINDIR/$bin"
}

for pair in "server:Server:cloudsh-server" "agent:Agent:cloudsh-agent" "client:Client:cloudsh"; do
    t="${pair%%:*}"; rest="${pair#*:}"; d="${rest%%:*}"; b="${rest##*:}"
    if [[ "$TARGET" == "all" || "$TARGET" == "$t" ]]; then
        install_bin "$t" "$b" || build_src "$t" "$d" "$b"
    fi
done

# systemd
if [[ "$(uname -s)" == "Linux" ]]; then
    if [[ "$TARGET" == "all" || "$TARGET" == "server" ]]; then
        sudo tee /etc/systemd/system/cloudsh-server.service >/dev/null <<EOF
[Unit]
Description=CloudSH Server
After=network.target
[Service]
Type=simple
WorkingDirectory=/opt/cloudsh
ExecStart=/usr/local/bin/node server.js
Restart=always
RestartSec=5
[Install]
WantedBy=multi-user.target
EOF
        echo -e "${GREEN}[systemd] cloudsh-server${NC}"
    fi
fi

echo -e "${GREEN}完成。${NC}"
echo "  Server:  cd /opt/cloudsh && node server.js"
echo "  Agent:   CLOUDSH_SERVER=$SERVER cloudsh-agent"
echo "  Client:  cloudsh -s $SERVER register"
