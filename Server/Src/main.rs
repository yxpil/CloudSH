/// CloudSH Server — 云中继
/// Agent 轮询拿命令，User 同步等结果
use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{oneshot, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

// ── 状态 ─────────────────────────────────────────────────

struct AgentState {
    password_hash: String,          // 常量时间比较用
    last_seen: u64,
    online: bool,
}

struct AppState {
    agents: RwLock<HashMap<String, AgentState>>,
    /// agent_id → 命令队列
    commands: RwLock<HashMap<String, VecDeque<PendingCommand>>>,
    /// command_id → oneshot sender（User 在等结果）
    waiters: RwLock<HashMap<String, oneshot::Sender<ExecResponse>>>,
}

use std::collections::VecDeque;

struct PendingCommand {
    command_id: String,
    command: String,
}

impl AppState {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            agents: RwLock::new(HashMap::new()),
            commands: RwLock::new(HashMap::new()),
            waiters: RwLock::new(HashMap::new()),
        })
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ── 密码工具 ─────────────────────────────────────────────

use rand::Rng;
use uuid::Uuid;

fn generate_password() -> String {
    let charset: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnpqrstuvwxyz23456789";
    let mut rng = rand::thread_rng();
    (0..16)
        .map(|_| charset[rng.gen_range(0..charset.len())] as char)
        .collect()
}

fn verify_password(given: &str, stored: &str) -> bool {
    given.len() == stored.len()
        && given
            .bytes()
            .zip(stored.bytes())
            .fold(0, |acc, (a, b)| acc | (a ^ b))
            == 0
}

// ── 请求/响应体 ──────────────────────────────────────────

#[derive(Deserialize)]
struct PollRequest {
    client_id: String,
    password: String,
}

#[derive(Serialize)]
struct PollResponse {
    command: Option<PollCommand>,
}

#[derive(Serialize)]
struct PollCommand {
    command_id: String,
    command: String,
}

#[derive(Deserialize)]
struct ResultRequest {
    client_id: String,
    password: String,
    command_id: String,
    stdout: String,
    stderr: String,
    exit_code: i32,
}

#[derive(Deserialize)]
struct ExecRequest {
    client_id: String,
    password: String,
    command: String,
}

#[derive(Serialize, Clone)]
struct ExecResponse {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

#[derive(Serialize)]
struct RegisterResponse {
    client_id: String,
    password: String,
}

// ── 响应辅助 ─────────────────────────────────────────────

fn ok_json(body: serde_json::Value) -> Response {
    (StatusCode::OK, Json(body)).into_response()
}

fn err_json(status: StatusCode, msg: &str) -> Response {
    (status, Json(serde_json::json!({"error": msg}))).into_response()
}

// ── 端点 ─────────────────────────────────────────────────

/// POST /register — Agent 或 User 注册
async fn register(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let client_id = Uuid::new_v4().to_string()[..8].to_string();
    let password = generate_password();

    state.agents.write().await.insert(
        client_id.clone(),
        AgentState {
            password_hash: password.clone(),
            last_seen: now_secs(),
            online: false,
        },
    );

    state
        .commands
        .write()
        .await
        .insert(client_id.clone(), VecDeque::new());

    ok_json(serde_json::to_value(&RegisterResponse {
        client_id,
        password,
    })
    .unwrap())
}

/// POST /agent/poll — Agent 心跳 + 拿命令
async fn agent_poll(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PollRequest>,
) -> impl IntoResponse {
    let agents = state.agents.read().await;
    let agent = match agents.get(&req.client_id) {
        Some(a) => a,
        None => return err_json(StatusCode::UNAUTHORIZED, "Unknown client_id"),
    };
    if !verify_password(&req.password, &agent.password_hash) {
        return err_json(StatusCode::UNAUTHORIZED, "Invalid password");
    }
    drop(agents);

    // 标记在线
    {
        let mut agents = state.agents.write().await;
        if let Some(a) = agents.get_mut(&req.client_id) {
            a.last_seen = now_secs();
            a.online = true;
        }
    }

    // 取下一个命令
    let cmd = {
        let mut queues = state.commands.write().await;
        queues
            .get_mut(&req.client_id)
            .and_then(|q| q.pop_front())
    };

    match cmd {
        Some(pending) => ok_json(serde_json::to_value(&PollResponse {
            command: Some(PollCommand {
                command_id: pending.command_id,
                command: pending.command,
            }),
        })
        .unwrap()),
        None => ok_json(serde_json::to_value(&PollResponse { command: None }).unwrap()),
    }
}

/// POST /agent/result — Agent 返回命令执行结果
async fn agent_result(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResultRequest>,
) -> impl IntoResponse {
    let agents = state.agents.read().await;
    let agent = match agents.get(&req.client_id) {
        Some(a) => a,
        None => return err_json(StatusCode::UNAUTHORIZED, "Unknown client_id"),
    };
    if !verify_password(&req.password, &agent.password_hash) {
        return err_json(StatusCode::UNAUTHORIZED, "Invalid password");
    }
    drop(agents);

    // 通知等待的 User
    let sender = state.waiters.write().await.remove(&req.command_id);
    if let Some(tx) = sender {
        let _ = tx.send(ExecResponse {
            stdout: req.stdout,
            stderr: req.stderr,
            exit_code: req.exit_code,
        });
    }

    ok_json(serde_json::json!({"status": "ok"}))
}

/// POST /exec — User 发送命令（同步等待 Agent 返回）
async fn exec(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecRequest>,
) -> impl IntoResponse {
    // 验证凭证
    let agents = state.agents.read().await;
    let agent = match agents.get(&req.client_id) {
        Some(a) => a,
        None => return err_json(StatusCode::UNAUTHORIZED, "Unknown client_id"),
    };
    if !verify_password(&req.password, &agent.password_hash) {
        return err_json(StatusCode::UNAUTHORIZED, "Invalid password");
    }
    if !agent.online {
        return err_json(StatusCode::SERVICE_UNAVAILABLE, "Agent offline");
    }
    drop(agents);

    // 字符安全
    if req.command.contains('\x00') || req.command.contains('\x1b') {
        return err_json(StatusCode::BAD_REQUEST, "Binary rejected — text only");
    }

    // 创建 oneshot 通道
    let command_id = Uuid::new_v4().to_string();
    let (tx, rx) = oneshot::channel();

    state
        .waiters
        .write()
        .await
        .insert(command_id.clone(), tx);

    // 入队命令
    {
        let mut queues = state.commands.write().await;
        queues.entry(req.client_id.clone()).or_default().push_back(PendingCommand {
            command_id: command_id.clone(),
            command: req.command,
        });
    }

    // 等待 Agent 返回（30 秒超时）
    match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
        Ok(Ok(resp)) => ok_json(serde_json::to_value(&resp).unwrap()),
        Ok(Err(_)) => {
            state.waiters.write().await.remove(&command_id);
            err_json(StatusCode::INTERNAL_SERVER_ERROR, "Channel closed")
        }
        Err(_) => {
            state.waiters.write().await.remove(&command_id);
            err_json(StatusCode::GATEWAY_TIMEOUT, "Agent did not respond in 30s")
        }
    }
}

// ── main ─────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    // --help / -h
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("CloudSH Server — 云中继");
        println!();
        println!("用法:");
        println!("  cloudsh-server");
        println!();
        println!("环境变量:");
        println!("  CLOUDSH_PORT  监听端口 (默认 8080)");
        println!();
        println!("端点:");
        println!("  POST /register       注册 Agent/User");
        println!("  POST /agent/poll     Agent 轮询拿命令");
        println!("  POST /agent/result   Agent 返回结果");
        println!("  POST /exec           User 同步执行命令");
        return;
    }

    tracing_subscriber::fmt::init();

    let state = AppState::new();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/register", post(register))
        .route("/agent/poll", post(agent_poll))
        .route("/agent/result", post(agent_result))
        .route("/exec", post(exec))
        .layer(cors)
        .with_state(state);

    let port: u16 = std::env::var("CLOUDSH_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let addr = format!("0.0.0.0:{port}");
    info!("CloudSH Server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
