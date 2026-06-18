#!/bin/bash
# CloudSH 一键安装 — 部署你自己的服务器
#
# curl -fsSL https://cloudsh.yxpil.com/install.sh | bash
# curl -fsSL https://cloudsh.yxpil.com/install.sh | bash -s server
# curl -fsSL https://cloudsh.yxpil.com/install.sh | bash -s agent
# CLOUDSH_SERVER=http://10.0.0.1:8080 bash install.sh agent  # 自配服务器

set -e
REPO="https://raw.githubusercontent.com/yxpil/CloudSH/main"
BINDIR="${INSTALL_DIR:-/usr/local/bin}"
SERVER="${CLOUDSH_SERVER:-http://localhost:8080}"

# 解析参数: install.sh [target] [--server URL]
TARGET="all"
for a in "$@"; do
    case "$a" in
        server|agent|client) TARGET="$a" ;;
        --server) ;;
        http*) SERVER="$a" ;;
    esac
done

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

RED='\033[0;31m'; GREEN='\033[0;32m'; CYAN='\033[0;36m'; NC='\033[0m'
echo -e "${CYAN}=== CloudSH Installer ===${NC}"
echo "  目标: $TARGET | 服务器: $SERVER"

if ! command -v cargo &>/dev/null; then
    echo -e "${GREEN}[Rust] 安装中...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

download() { curl -fsSL "$REPO/$1/$2" -o "$TMPDIR/$1/$2"; }

build() {
    local name="$1" dir="$2" bin="$3"
    echo -e "${GREEN}[编译] $name${NC}"
    mkdir -p "$TMPDIR/$dir/Src"
    download "$dir" Cargo.toml
    download "$dir" "Src/main.rs"
    cd "$TMPDIR/$dir"
    cargo build --release -q 2>&1 | tail -1
    sudo cp "target/release/$bin" "$BINDIR/$bin"
    sudo chmod +x "$BINDIR/$bin"
    echo "  -> $BINDIR/$bin"
}

[[ "$TARGET" == "all" || "$TARGET" == "server" ]] && build "Server" "Server" "cloudsh-server"
[[ "$TARGET" == "all" || "$TARGET" == "agent" ]]  && build "Agent"  "Agent"  "cloudsh-agent"
[[ "$TARGET" == "all" || "$TARGET" == "client" ]] && build "Client" "Client" "cloudsh"

# systemd
if [[ "$(uname -s)" == "Linux" ]]; then
    echo -e "${GREEN}[systemd]${NC}"
    if [[ "$TARGET" == "all" || "$TARGET" == "server" ]]; then
        sudo tee /etc/systemd/system/cloudsh-server.service >/dev/null <<EOF
[Unit]
Description=CloudSH Server
After=network.target
[Service]
Type=simple
ExecStart=$BINDIR/cloudsh-server
Environment=CLOUDSH_PORT=8080
Restart=always
RestartSec=5
[Install]
WantedBy=multi-user.target
EOF
        echo "  systemctl enable --now cloudsh-server"
    fi
    if [[ "$TARGET" == "all" || "$TARGET" == "agent" ]]; then
        sudo tee /etc/systemd/system/cloudsh-agent.service >/dev/null <<EOF
[Unit]
Description=CloudSH Agent
After=network.target
[Service]
Type=simple
ExecStart=$BINDIR/cloudsh-agent
Environment=CLOUDSH_SERVER=$SERVER
Environment=CLOUDSH_CLIENT_ID=YOUR_CLIENT_ID
Environment=CLOUDSH_PASSWORD=YOUR_PW
Restart=always
RestartSec=10
[Install]
WantedBy=multi-user.target
EOF
        echo "  vi /etc/systemd/system/cloudsh-agent.service  # 填 YOUR_CLIENT_ID / YOUR_PW"
    fi
fi

echo ""
echo -e "${GREEN}完成。${NC}"
echo "  Server:  CLOUDSH_PORT=8080 cloudsh-server"
echo "  Agent:   CLOUDSH_SERVER=$SERVER cloudsh-agent"
echo "  Client:  cloudsh -s $SERVER register"
