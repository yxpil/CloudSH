/// CloudSH Client — 用户 CLI
/// 向中继发送命令，同步等待 Agent 返回结果
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

// ── CLI ──────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "cloudsh", about = "CloudSH — 用户客户端")]
struct Cli {
    /// 中继服务器地址
    #[arg(short = 's', long, default_value = "http://localhost:8080")]
    server: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 注册（获取被控主机的 client_id + password）
    Register,

    /// 执行命令（同步等待被控主机返回）
    Exec {
        /// 要执行的命令
        command: Vec<String>,
    },

    /// 查看当前状态
    Status,
}

// ── 本地状态 ─────────────────────────────────────────────

#[derive(Serialize, Deserialize, Default, Clone)]
struct ClientState {
    client_id: String,
    password: String,
    server_url: String,
}

fn state_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cloudsh")
        .join("state.json")
}

fn load_state() -> ClientState {
    let path = state_path();
    if path.exists() {
        let data = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        ClientState::default()
    }
}

fn save_state(state: &ClientState) {
    let path = state_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&path, serde_json::to_string_pretty(state).unwrap()).ok();
}

// ── API ──────────────────────────────────────────────────

async fn api_register(client: &HttpClient, server: &str) -> Result<ClientState, String> {
    let resp = client
        .post(format!("{server}/register"))
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析: {e}"))?;
    let cid = json["client_id"].as_str().unwrap_or("").to_string();
    let pw = json["password"].as_str().unwrap_or("").to_string();

    if cid.is_empty() {
        return Err(format!("注册失败: {json}"));
    }

    Ok(ClientState {
        client_id: cid,
        password: pw,
        server_url: server.to_string(),
    })
}

async fn api_exec(
    client: &HttpClient,
    state: &ClientState,
    command: &str,
) -> Result<(String, String, i32), String> {
    let body = serde_json::json!({
        "client_id": state.client_id,
        "password": state.password,
        "command": command,
    });

    let resp = client
        .post(format!("{}/exec", state.server_url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| format!("解析: {e}"))?;

    if let Some(err) = json["error"].as_str() {
        return Err(err.to_string());
    }

    Ok((
        json["stdout"].as_str().unwrap_or("").to_string(),
        json["stderr"].as_str().unwrap_or("").to_string(),
        json["exit_code"].as_i64().unwrap_or(-1) as i32,
    ))
}

// ── main ─────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let client = HttpClient::new();
    let mut state = load_state();

    let result = match &cli.command {
        Command::Register => {
            match api_register(&client, &cli.server).await {
                Ok(s) => {
                    state = s;
                    save_state(&state);
                    println!("client_id:  {}", state.client_id);
                    println!("password:   {}", state.password);
                    println!();
                    println!("凭证已保存到 ~/.cloudsh/state.json");
                    println!("在被控主机上执行:  CLOUDSH_SERVER={} cloudsh-agent", cli.server);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }

        Command::Exec { command } => {
            if state.client_id.is_empty() {
                eprintln!("请先注册: cloudsh register -s {}", cli.server);
                return;
            }
            if state.server_url != cli.server {
                state.server_url = cli.server.clone();
                save_state(&state);
            }
            let cmd = command.join(" ");
            match api_exec(&client, &state, &cmd).await {
                Ok((stdout, stderr, code)) => {
                    if !stdout.is_empty() {
                        print!("{stdout}");
                    }
                    if !stderr.is_empty() {
                        eprint!("{stderr}");
                    }
                    if code != 0 {
                        eprintln!("exit: {code}");
                    }
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }

        Command::Status => {
            if state.client_id.is_empty() {
                println!("未注册 — cloudsh register -s {}", cli.server);
            } else {
                println!("client_id:  {}", state.client_id);
                println!("server:     {}", state.server_url);
            }
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("错误: {e}");
        std::process::exit(1);
    }
}
