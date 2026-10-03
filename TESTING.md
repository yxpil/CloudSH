# CloudSH 测试说明

CloudSH 是三 crate 工程（无根 Cargo.toml，各自独立 `cargo test`）：

| crate | 目录 | 说明 |
|---|---|---|
| `cloudsh-server` | `Server/` | 云中继（axum HTTP API） |
| `cloudsh-agent` | `Agent/` | 被控主机守护（轮询并本地执行命令） |
| `cloudsh-client` | `Client/` | 用户 CLI（clap） |

## 运行命令（在各 crate 目录内执行）

```powershell
cd Server; cargo test
cd Agent;  cargo test
cd Client; cargo test
```

## 覆盖清单

### Server（`Server/Src/main.rs` 内单元 3 个 + `Server/tests/server_http.rs` 集成 1 个）

单元：
- `generated_password_is_16_safe_chars_and_unique`：密码 16 位、仅安全字符集、两次不重复。
- `verify_password_accepts_exact_rejects_others`：正确密码通过；错误/异长/差一位均拒绝。
- `now_secs_is_reasonable_unix_time`。

集成（真实 HTTP，裸 TcpStream，不引入额外依赖）：
- `register_auth_and_injection_rejection`：`/register` 返回 8 位 client_id + 16 位 password；
  错误密码 / 未知 client_id → **401**；agent 上线后发送含 NUL(`\u0000`) / ANSI ESC(`\u001b`) 的
  命令 → **400**（“text only”过滤）；全新未上线 agent 发命令 → **503**（立即返回，不排队等待）。

### Agent（`Agent/Src/main.rs` 内单元 2 个 + `Agent/tests/help_smoke.rs` 集成 1 个）

单元：
- `execute_simple_echo_returns_stdout`：`echo cloudsh-works` → stdout 含该串、exit 0。
- `execute_unknown_command_nonzero_exit`：不存在的命令 → 非零退出码。

集成：
- `agent_help_exits_zero`：`cloudsh-agent --help` 退出 0 且打印帮助。

### Client（`Client/tests/cli_smoke.rs` 集成 3 个）

- `client_help_exits_zero`：`cloudsh --help` 退出 0。
- `status_without_register_reports_unregistered`：隔离 HOME 下未注册 → 打印“未注册”、不崩溃。
- `exec_without_register_exits_nonzero_with_hint`：未注册直接 exec → 提示先注册、不发网络请求。

## 注入测试

- **认证注入**：错误密码 / 未知 client_id / 爆破单字符均被 `verify_password` 拒绝 → 401。
- **控制字符注入**：经 JSON `\u0000` / `\u001b` 传入的 NUL / ANSI 转义序列命令被服务端
  “text only”检查拦截 → 400，不会进入命令队列。
- **命令执行面**：Agent 端 `execute_command` 真实执行命令（这是被控端的本职功能），
  测试用 `echo` 与不存在命令验证返回值解析正确。

## 钩子 / 事件

CloudSH 无显式插件/回调注册表；其“请求→处理→响应”链路在 Server 集成测试中按顺序验证：
register → poll(上线) → exec(过滤/排队) → result(oneshot 唤醒)。一个 agent 的命令被正确路由到
它自己的队列，不影响其他注册（离线 agent 立即 503，不占用在线 agent 的等待通道）。

## 预期结果

- `cd Server; cargo test`：3 passed（单元）+ 1 passed（集成）。
- `cd Agent; cargo test`：2 passed（单元）+ 1 passed（集成）。
- `cd Client; cargo test`：3 passed（集成）。
