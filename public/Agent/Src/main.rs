/// CloudSH Agent — 被控主机守护进程
/// 通过环境变量拿凭证（由 User 先注册好），
/// 或自动注册。轮询命令 → 本地执行 → 返回结果。
use std::path::PathBuf;
use std::process::Command as SysCommand;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

// ── 本地缓存 ─────────────────────────────────────────────

#[derive(Serialize, Deserialize, Default)]
struct AgentState {
    client_id: String,
    password: String,
    server_url: String,
}

fn state_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cloudsh-agent")
        .join("state.json")
}

fn load_state() -> AgentState {
    let path = state_path();
    if path.exists() {
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        AgentState::default()
    }
}

fn save_state(state: &AgentState) {
    let path = state_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&path, serde_json::to_string_pretty(state).unwrap()).ok();
}

// ── API ──────────────────────────────────────────────────

async fn register(client: &Client, server: &str) -> Result<AgentState, String> {
    let resp = client
        .post(format!("{server}/register"))
        .send()
        .await
        .map_err(|e| format!("注册失败: {e}"))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析: {e}"))?;
    let cid = json["client_id"].as_str().unwrap_or("").to_string();
    let pw = json["password"].as_str().unwrap_or("").to_string();

    if cid.is_empty() {
        return Err(format!("注册失败: {json}"));
    }

    Ok(AgentState {
        client_id: cid,
        password: pw,
        server_url: server.to_string(),
    })
}

#[derive(Deserialize)]
struct PollResponse {
    command: Option<PollCommand>,
}

#[derive(Deserialize)]
struct PollCommand {
    command_id: String,
    command: String,
}

async fn poll(client: &Client, state: &AgentState) -> Result<Option<(String, String)>, String> {
    let body = serde_json::json!({
        "client_id": state.client_id,
        "password": state.password,
    });

    let resp = client
        .post(format!("{}/agent/poll", state.server_url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("poll: {e}"))?;

    let json: PollResponse = resp.json().await.map_err(|e| format!("解析: {e}"))?;

    match json.command {
        Some(cmd) => Ok(Some((cmd.command_id, cmd.command))),
        None => Ok(None),
    }
}

async fn send_result(
    client: &Client,
    state: &AgentState,
    command_id: &str,
    stdout: &str,
    stderr: &str,
    exit_code: i32,
) -> Result<(), String> {
    let body = serde_json::json!({
        "client_id": state.client_id,
        "password": state.password,
        "command_id": command_id,
        "stdout": stdout,
        "stderr": stderr,
        "exit_code": exit_code,
    });

    client
        .post(format!("{}/agent/result", state.server_url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("发送结果失败: {e}"))?;

    Ok(())
}

// ── 本地执行 ─────────────────────────────────────────────

fn execute_command(command: &str) -> (String, String, i32) {
    let output = if cfg!(target_os = "windows") {
        SysCommand::new("cmd").args(["/C", command]).output()
    } else {
        SysCommand::new("sh").args(["-c", command]).output()
    };

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let code = out.status.code().unwrap_or(-1);
            (stdout, stderr, code)
        }
        Err(e) => (String::new(), format!("执行失败: {e}"), -1),
    }
}

// ── main ─────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    // --help / -h
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("CloudSH Agent — 被控主机守护进程");
        println!();
        println!("用法:");
        println!("  cloudsh-agent");
        println!();
        println!("环境变量:");
        println!("  CLOUDSH_SERVER       中继服务器地址 (默认 http://localhost:8080)");
        println!("  CLOUDSH_CLIENT_ID    客户端 ID (不设则自动注册)");
        println!("  CLOUDSH_PASSWORD     密码 (不设则自动注册)");
        println!();
        println!("Agent 连接到中继服务器，轮询命令并本地执行。");
        println!("在 User 端用 'cloudsh register' 获取凭证，");
        println!("然后传给 Agent 的环境变量。");
        return;
    }

    let server_url =
        std::env::var("CLOUDSH_SERVER").unwrap_or_else(|_| "http://localhost:8080".to_string());

    let mut state = load_state();

    // 优先从环境变量拿凭证（User 已注册，直接复用）
    let env_id = std::env::var("CLOUDSH_CLIENT_ID").ok();
    let env_pw = std::env::var("CLOUDSH_PASSWORD").ok();

    if let (Some(cid), Some(pw)) = (env_id, env_pw) {
        state = AgentState {
            client_id: cid,
            password: pw,
            server_url: server_url.clone(),
        };
        save_state(&state);
    }

    // 如果没凭证，自动注册
    if state.client_id.is_empty() {
        let client = Client::new();
        eprintln!("正在注册到 {server_url} ...");
        match register(&client, &server_url).await {
            Ok(s) => {
                state = s;
                save_state(&state);
                eprintln!("注册成功");
            }
            Err(e) => {
                eprintln!("注册失败: {e}");
                return;
            }
        }
    }

    let client = Client::new();
    eprintln!("CloudSH Agent 已启动");
    eprintln!("  client_id: {}", state.client_id);
    eprintln!("  server:    {}", state.server_url);
    eprintln!("  轮询间隔: 2s");
    eprintln!();

    loop {
        match poll(&client, &state).await {
            Ok(Some((cmd_id, command))) => {
                eprintln!("[执行] {command}");
                let (stdout, stderr, code) = execute_command(&command);

                if let Err(e) =
                    send_result(&client, &state, &cmd_id, &stdout, &stderr, code).await
                {
                    eprintln!("发送结果失败: {e}");
                } else {
                    if !stdout.is_empty() {
                        eprintln!("[stdout] {}", stdout.trim());
                    }
                    if !stderr.is_empty() {
                        eprintln!("[stderr] {}", stderr.trim());
                    }
                    eprintln!("[完成] exit={code}");
                }
            }
            Ok(None) => {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(e) => {
                eprintln!("轮询错误: {e}");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
