#!/bin/bash
# CloudSH 一键安装脚本
# 用法:
#   bash install.sh              # 编译全部三个
#   bash install.sh server       # 只编译 Server（云 VPS）
#   bash install.sh agent        # 只编译 Agent（被控主机）
#   bash install.sh client       # 只编译 Client（用户机）

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

TARGET="${1:-all}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

echo -e "${CYAN}╔══════════════════════════════════╗${NC}"
echo -e "${CYAN}║     CloudSH Installer           ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════╝${NC}"
echo ""

# ── 检查 Rust ────────────────────────────────────────────

if ! command -v cargo &>/dev/null; then
    echo -e "${GREEN}[1/3] 安装 Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo -e "${GREEN}[*] Rust 已安装: $(rustc --version)${NC}"
fi

# ── 编译 ──────────────────────────────────────────────────

build_component() {
    local name="$1"
    local dir="$2"
    local bin="$3"
    echo -e "${GREEN}[编译] $name ...${NC}"
    cd "$SCRIPT_DIR/$dir"
    cargo build --release 2>&1 | tail -3
    cp "target/release/$bin" "$SCRIPT_DIR/bin/$bin" 2>/dev/null || true
    echo -e "${GREEN}  -> $SCRIPT_DIR/$dir/target/release/$bin${NC}"
}

mkdir -p "$SCRIPT_DIR/bin"

case "$TARGET" in
    all|server)
        build_component "Server (云中继)" "Server" "cloudsh"
        ;;
esac

case "$TARGET" in
    all|agent)
        build_component "Agent (被控主机)" "Agent" "cloudsh-agent"
        ;;
esac

case "$TARGET" in
    all|client)
        build_component "Client (用户)" "Client" "cloudsh"
        ;;
esac

# ── 安装二进制 ────────────────────────────────────────────

echo ""
echo -e "${GREEN}[2/3] 安装二进制到 $INSTALL_DIR ...${NC}"

case "$TARGET" in
    all|server)
        sudo cp "$SCRIPT_DIR/Server/target/release/cloudsh" "$INSTALL_DIR/cloudsh-server"
        sudo chmod +x "$INSTALL_DIR/cloudsh-server"
        echo "  cloudsh-server -> $INSTALL_DIR/cloudsh-server"
        ;;
esac

case "$TARGET" in
    all|agent)
        sudo cp "$SCRIPT_DIR/Agent/target/release/cloudsh-agent" "$INSTALL_DIR/cloudsh-agent"
        sudo chmod +x "$INSTALL_DIR/cloudsh-agent"
        echo "  cloudsh-agent  -> $INSTALL_DIR/cloudsh-agent"
        ;;
esac

case "$TARGET" in
    all|client)
        sudo cp "$SCRIPT_DIR/Client/target/release/cloudsh" "$INSTALL_DIR/cloudsh"
        sudo chmod +x "$INSTALL_DIR/cloudsh"
        echo "  cloudsh        -> $INSTALL_DIR/cloudsh"
        ;;
esac

# ── systemd 服务文件（仅 Linux）────────────────────────────

if [[ "$(uname -s)" == "Linux" ]]; then
    echo ""
    echo -e "${GREEN}[3/3] 生成 systemd 服务文件...${NC}"

    if [[ "$TARGET" == "all" || "$TARGET" == "server" ]]; then
        sudo tee /etc/systemd/system/cloudsh-server.service > /dev/null <<'SERVICE'
[Unit]
Description=CloudSH Relay Server
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/cloudsh-server
Environment=CLOUDSH_PORT=8080
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
SERVICE
        echo "  /etc/systemd/system/cloudsh-server.service"
    fi

    if [[ "$TARGET" == "all" || "$TARGET" == "agent" ]]; then
        sudo tee /etc/systemd/system/cloudsh-agent.service > /dev/null <<'SERVICE'
[Unit]
Description=CloudSH Agent (被控主机)
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/cloudsh-agent
Environment=CLOUDSH_SERVER=http://YOUR_SERVER:8080
Environment=CLOUDSH_CLIENT_ID=YOUR_CLIENT_ID
Environment=CLOUDSH_PASSWORD=YOUR_PASSWORD
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
SERVICE
        echo "  /etc/systemd/system/cloudsh-agent.service"
        echo ""
        echo -e "${RED}  ⚠ 编辑 /etc/systemd/system/cloudsh-agent.service${NC}"
        echo -e "${RED}     替换 YOUR_SERVER / YOUR_CLIENT_ID / YOUR_PASSWORD${NC}"
    fi

    echo ""
    echo "启用服务:"
    [[ "$TARGET" == "all" || "$TARGET" == "server" ]] && echo "  sudo systemctl enable --now cloudsh-server"
    [[ "$TARGET" == "all" || "$TARGET" == "agent" ]]  && echo "  sudo systemctl enable --now cloudsh-agent   (先编辑 Environment)"
else
    echo ""
    echo -e "${CYAN}[3/3] 跳过 (macOS — 用 launchd 或直接运行)${NC}"
fi

# ── 完成 ──────────────────────────────────────────────────

echo ""
echo -e "${GREEN}╔══════════════════════════════════╗${NC}"
echo -e "${GREEN}║     安装完成！                  ║${NC}"
echo -e "${GREEN}╚══════════════════════════════════╝${NC}"
echo ""
echo "快速上手:"
echo ""
echo "  1. 云 VPS 启动 Server:"
echo "     CLOUDSH_PORT=8080 cloudsh-server"
echo ""
echo "  2. 用户注册:"
echo "     cloudsh -s http://<VPS_IP>:8080 register"
echo "     # 记下 client_id 和 password"
echo ""
echo "  3. 被控主机启动 Agent:"
echo "     CLOUDSH_SERVER=http://<VPS_IP>:8080 \\"
echo "     CLOUDSH_CLIENT_ID=<client_id> \\"
echo "     CLOUDSH_PASSWORD=<password> \\"
echo "     cloudsh-agent"
echo ""
echo "  4. 用户发送命令:"
echo "     cloudsh -s http://<VPS_IP>:8080 exec whoami"
echo ""
echo "  最多支持机器数: 理论上无限（内存/连接限制）"
echo "  实测数千台没问题"
