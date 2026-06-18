# CloudSH

云 SSH 中继 — 通过 HTTP 转发命令到被控主机，无需公网 IP，无需开放端口。

```
你 (任何网络)         云 VPS              被控主机 (内网)
cloudsh exec ──HTTP──→ Server ←──poll── cloudsh-agent
    ↑                    │                     │
    │                    │   命令入队           │ 本地执行
    │                    │                     │
    └── 同步等待 ←──── 转发结果 ←──── result ──┘
```

## 架构

| 组件 | 部署位置 | 作用 |
|------|---------|------|
| **Server** | 云 VPS | HTTP API 中继，命令队列 + 结果转发 |
| **Agent** | 被控主机 | 轮询命令，本地执行，返回文本 |
| **Client** | 用户机 | CLI，注册 / 执行命令 / 查看状态 |

## 特性

- 纯文本转发（拒绝二进制和控制字符）
- 被控主机无需公网 IP、无需开放 SSH 端口
- Agent 主动外连，穿透任意防火墙/NAT
- 常量时间密码比较，防时序攻击
- 内存存储，无持久化依赖，重启即清
- 最多支持数千台被控主机

## 安装

```bash
git clone https://github.com/yxpil/CloudSH
cd CloudSH

# 一键编译全部
bash install.sh

# 或分机器编译
bash install.sh server   # 云 VPS 上
bash install.sh agent    # 被控主机上
bash install.sh client   # 用户机上
```

需要 Rust 1.80+。脚本会自动安装。

## 快速开始

### 1. 启动 Server（云 VPS）

```bash
CLOUDSH_PORT=8080 cloudsh-server
```

### 2. 注册 + 获取凭证（本地）

```bash
cloudsh -s http://<VPS_IP>:8080 register
# client_id:  a1b2c3d4
# password:   xxxxxxxxxxxxxxxx
```

### 3. 启动 Agent（被控主机）

```bash
CLOUDSH_SERVER=http://<VPS_IP>:8080 \
CLOUDSH_CLIENT_ID=a1b2c3d4 \
CLOUDSH_PASSWORD=xxxxxxxxxxxxxxxx \
cloudsh-agent
```

### 4. 执行命令

```bash
cloudsh -s http://<VPS_IP>:8080 exec whoami
cloudsh -s http://<VPS_IP>:8080 exec "ls -la /var/log"
cloudsh -s http://<VPS_IP>:8080 exec "df -h && free -m"
```

## API

所有端点均为 `POST`，`Content-Type: application/json`。

### POST /register

注册，返回 client_id + password。

```
→ (空)
← {"client_id": "a1b2c3d4", "password": "xxxxxxxxxxxxxxxx"}
```

### POST /agent/poll

Agent 心跳 + 获取待执行命令。

```
→ {"client_id": "...", "password": "..."}
← {"command": {"command_id": "xxx", "command": "whoami"}}
← {"command": null}  // 无待执行命令
```

### POST /agent/result

Agent 返回命令执行结果。

```
→ {"client_id": "...", "password": "...", "command_id": "xxx",
    "stdout": "...", "stderr": "...", "exit_code": 0}
← {"status": "ok"}
```

### POST /exec

用户同步执行命令（阻塞等待 Agent 返回，30s 超时）。

```
→ {"client_id": "...", "password": "...", "command": "whoami"}
← {"stdout": "root\n", "stderr": "", "exit_code": 0}
← {"error": "Agent did not respond in 30s"}
```

## 环境变量

### Server

| 变量 | 默认 | 说明 |
|------|-----|------|
| `CLOUDSH_PORT` | 8080 | HTTP 监听端口 |

### Agent

| 变量 | 默认 | 说明 |
|------|-----|------|
| `CLOUDSH_SERVER` | http://localhost:8080 | Server 地址 |
| `CLOUDSH_CLIENT_ID` | (自动注册) | 客户端 ID |
| `CLOUDSH_PASSWORD` | (自动注册) | 密码 |

### Client

| 变量 | 默认 | 说明 |
|------|-----|------|
| `-s / --server` | http://localhost:8080 | Server 地址（CLI 参数） |

## systemd（Linux）

`install.sh` 自动生成服务文件：

```bash
# Server
sudo systemctl enable --now cloudsh-server

# Agent（先编辑 Environment 替换凭证）
sudo vi /etc/systemd/system/cloudsh-agent.service
sudo systemctl enable --now cloudsh-agent
```

## 安全

- 密码用常量时间比较，防时序攻击
- 命令拒绝 `\x00`（NULL）和 `\x1b`（ESC），只过纯文本
- Server 不存储 SSH 凭证，Agent 本地执行命令
- 30 秒执行超时，防止资源耗尽
- 建议生产环境加 TLS 反向代理（nginx/caddy）

## License

MIT
