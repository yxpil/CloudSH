//! CloudSH Server HTTP 集成测试（用裸 TcpStream，不引入额外依赖）
//!
//! 拉起真实服务器（CLOUDSH_PORT 指定空闲端口），通过真实 HTTP 验证：
//!   * /register 返回 client_id + password
//!   * 错误密码 / 未知 client_id -> 401（认证不被绕过）
//!   * /exec 拒绝含 NUL / ESC 的"二进制"命令 -> 400（注入/转义序列过滤）
//!   * Agent 离线时 /exec -> 503
//!
//! 运行：cargo test --test server_http -- --nocapture

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const PORT: u16 = 18081;

struct Svr {
    child: std::process::Child,
}
impl Drop for Svr {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn() -> Svr {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cloudsh"));
    cmd.env("CLOUDSH_PORT", PORT.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    Svr {
        child: cmd.spawn().expect("启动 cloudsh-server 失败"),
    }
}

/// 发一个 POST JSON，返回 (status_code, body_text)
fn post(path: &str, body: &str) -> (u16, String) {
    let addr = format!("127.0.0.1:{PORT}");
    let mut stream = TcpStream::connect(&addr).expect("连不上 server");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).unwrap();
    let text = String::from_utf8_lossy(&buf).to_string();
    let status = text
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body_text = text.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body_text)
}

fn wait_up() {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if post("/register", "{}").0 != 0 {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("server 未就绪");
}

#[test]
fn register_auth_and_injection_rejection() {
    let _svr = spawn();
    wait_up();

    // 1) 注册
    let (_st, body) = post("/register", "{}");
    let reg: serde_json::Value = serde_json::from_str(&body).unwrap();
    let cid = reg["client_id"].as_str().unwrap().to_string();
    let pw = reg["password"].as_str().unwrap().to_string();
    assert_eq!(cid.len(), 8);
    assert_eq!(pw.len(), 16);

    // 2) 错误密码 poll -> 401
    let (st, _) = post(
        "/agent/poll",
        &format!(r#"{{"client_id":"{cid}","password":"wrong-pass"}}"#),
    );
    assert_eq!(st, 401);

    // 3) 未知 client_id -> 401
    let (st, _) = post("/agent/poll", r#"{"client_id":"nobody","password":"x"}"#);
    assert_eq!(st, 401);

    // 4) 让 agent 上线（poll 一次），再发含 NUL/ESC 的命令 -> 400
    //    （服务端先判离线 503，再判二进制 400，所以必须先在线）
    let (_st, _) = post(
        "/agent/poll",
        &format!(r#"{{"client_id":"{cid}","password":"{pw}"}}"#),
    );
    for cmd in [r#"echo hello\u0000"#, r#"ls; id\u001b[0m"#] {
        let (st, _) = post(
            "/exec",
            &format!(r#"{{"client_id":"{cid}","password":"{pw}","command":"{cmd}"}}"#),
        );
        assert_eq!(st, 400, "二进制命令应被拒: {cmd:?}");
    }

    // 5) 另注册一个全新 agent（从不 poll，离线），普通命令 -> 503（不进入排队等待）
    let (_st, body) = post("/register", "{}");
    let reg2: serde_json::Value = serde_json::from_str(&body).unwrap();
    let cid2 = reg2["client_id"].as_str().unwrap();
    let pw2 = reg2["password"].as_str().unwrap();
    let (st, _) = post(
        "/exec",
        &format!(r#"{{"client_id":"{cid2}","password":"{pw2}","command":"$(reboot)"}}"#),
    );
    assert_eq!(st, 503, "离线 agent 应立即 503，不等待");

    // 6) 正确密码 poll（在线 agent）-> 200 且无命令
    let (st, body) = post(
        "/agent/poll",
        &format!(r#"{{"client_id":"{cid}","password":"{pw}"}}"#),
    );
    assert_eq!(st, 200);
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["command"], serde_json::Value::Null);
}
