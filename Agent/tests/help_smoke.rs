//! CloudSH Agent CLI 冒烟测试：--help 正常退出。
//! 运行：cargo test --test help_smoke

#[test]
fn agent_help_exits_zero() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_cloudsh-agent"))
        .arg("--help")
        .output()
        .expect("启动 cloudsh-agent 失败");
    assert!(out.status.success(), "agent --help 应退出 0");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("CloudSH Agent"), "应打印帮助: {text}");
}
