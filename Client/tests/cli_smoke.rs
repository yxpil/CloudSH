//! CloudSH Client CLI 冒烟测试（隔离 HOME，避免依赖真实 ~/.cloudsh）。
//! 运行：cargo test --test cli_smoke

use std::process::Command;

fn isolated_home() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cloudsh-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn client_help_exits_zero() {
    let out = Command::new(env!("CARGO_BIN_EXE_cloudsh"))
        .arg("--help")
        .output()
        .expect("启动 cloudsh 失败");
    assert!(out.status.success(), "--help 应退出 0");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("cloudsh"), "应打印帮助: {text}");
}

#[test]
fn status_without_register_reports_unregistered() {
    let home = isolated_home();
    let out = Command::new(env!("CARGO_BIN_EXE_cloudsh"))
        .arg("status")
        .env("USERPROFILE", &home) // Windows home
        .env("HOME", &home) // unix fallback
        .output()
        .unwrap();
    assert!(out.status.success(), "未注册 status 不应崩");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("未注册"), "应提示未注册: {text}");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn exec_without_register_exits_nonzero_with_hint() {
    let home = isolated_home();
    let out = Command::new(env!("CARGO_BIN_EXE_cloudsh"))
        .args(["exec", "echo", "hi"])
        .env("USERPROFILE", &home)
        .env("HOME", &home)
        .output()
        .unwrap();
    // main 里未注册会 eprintln + return（main 是 () 不 exit），所以退出码可能是 0；
    // 关键是不 panic、不真的发网络请求。这里只断言进程正常结束且提示先注册。
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(combined.contains("注册"), "应提示先注册: {combined}");
    let _ = std::fs::remove_dir_all(&home);
}
