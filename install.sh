#!/bin/bash
# curl -fsSL https://cloudsh.yxpil.com/install.sh | bash        # 全装
# curl -fsSL https://cloudsh.yxpil.com/install.sh | bash -s server  # 仅 Server
set -e

TARGET="${1:-all}"
REPO="https://raw.githubusercontent.com/yxpil/CloudSH/main"
TMPDIR=$(mktemp -d)
BINDIR="${INSTALL_DIR:-/usr/local/bin}"
trap 'rm -rf "$TMPDIR"' EXIT

RED='\033[0;31m'; GREEN='\033[0;32m'; CYAN='\033[0;36m'; NC='\033[0m'

echo -e "${CYAN}=== CloudSH Installer ===${NC}"

# Rust
if ! command -v cargo &>/dev/null; then
    echo -e "${GREEN}[1/4] Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# 下载源码
download() { curl -fsSL "$REPO/$1/$2" -o "$TMPDIR/$1/$2"; }

build() {
    local name="$1" dir="$2" bin="$3" files="$4"
    echo -e "${GREEN}[编译] $name${NC}"
    mkdir -p "$TMPDIR/$dir/Src"
    download "$dir" Cargo.toml
    for f in $files; do download "$dir" "Src/$f"; done
    cd "$TMPDIR/$dir"
    cargo build --release -q 2>&1 | tail -1
    sudo cp "target/release/$bin" "$BINDIR/$bin"
    sudo chmod +x "$BINDIR/$bin"
    echo "  -> $BINDIR/$bin"
}

if [[ "$TARGET" == "all" || "$TARGET" == "server" ]]; then
    build "Server" "Server" "cloudsh-server" "main.rs"
fi
if [[ "$TARGET" == "all" || "$TARGET" == "agent" ]]; then
    build "Agent" "Agent" "cloudsh-agent" "main.rs"
fi
if [[ "$TARGET" == "all" || "$TARGET" == "client" ]]; then
    build "Client" "Client" "cloudsh" "main.rs"
fi

# systemd (Linux only)
if [[ "$(uname -s)" == "Linux" ]]; then
    echo -e "${GREEN}[systemd]${NC}"
    if [[ "$TARGET" == "all" || "$TARGET" == "server" ]]; then
        sudo tee /etc/systemd/system/cloudsh-server.service >/dev/null <<'EOF'
[Unit]
Description=CloudSH Server
After=network.target
[Service]
Type=simple
ExecStart=/usr/local/bin/cloudsh-server
Environment=CLOUDSH_PORT=8080
Restart=always
RestartSec=5
[Install]
WantedBy=multi-user.target
EOF
        echo "  systemctl enable --now cloudsh-server"
    fi
    if [[ "$TARGET" == "all" || "$TARGET" == "agent" ]]; then
        sudo tee /etc/systemd/system/cloudsh-agent.service >/dev/null <<'EOF'
[Unit]
Description=CloudSH Agent
After=network.target
[Service]
Type=simple
ExecStart=/usr/local/bin/cloudsh-agent
Environment=CLOUDSH_SERVER=http://YOUR_SERVER:8080
Environment=CLOUDSH_CLIENT_ID=YOUR_ID
Environment=CLOUDSH_PASSWORD=YOUR_PW
Restart=always
RestartSec=10
[Install]
WantedBy=multi-user.target
EOF
        echo "  vi /etc/systemd/system/cloudsh-agent.service  # 改凭证"
    fi
fi

echo -e "${GREEN}完成。${NC}"
