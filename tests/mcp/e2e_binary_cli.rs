//! Tests for the `devlogger-mcp` binary's own CLI surface — `--help`,
//! `--version`, and `--dir`.  These matter even for an MCP-only binary:
//! package managers, lockfile generators, and CI smoke tests shell out
//! to `--version` or `--help`, and a broken flag blocks those.

use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_devlogger-mcp")
}

#[tokio::test]
async fn help_flag_prints_usage_and_exits_zero() {
    let out = Command::new(bin())
        .arg("--help")
        .output()
        .await
        .expect("spawn");
    assert!(out.status.success(), "--help should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Usage:"), "missing Usage section: {stdout}");
    assert!(
        stdout.contains("--dir"),
        "--help should document --dir: {stdout}"
    );
}

#[tokio::test]
async fn short_help_flag_works() {
    let out = Command::new(bin())
        .arg("-h")
        .output()
        .await
        .expect("spawn");
    assert!(out.status.success(), "-h should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Usage:"), "missing Usage: {stdout}");
}

#[tokio::test]
async fn version_flag_prints_crate_version() {
    let out = Command::new(bin())
        .arg("--version")
        .output()
        .await
        .expect("spawn");
    assert!(out.status.success(), "--version should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "--version should print CARGO_PKG_VERSION; got {stdout}"
    );
}

#[tokio::test]
async fn unknown_flag_exits_nonzero() {
    let out = Command::new(bin())
        .arg("--this-flag-does-not-exist")
        .output()
        .await
        .expect("spawn");
    assert!(!out.status.success(), "unknown flag should exit nonzero");
}

#[tokio::test]
async fn dir_flag_is_respected_by_server() {
    // End-to-end: spawn with --dir <tmp> and send one initialize + one
    // devlog_new, then confirm the file landed in <tmp>.  We speak raw
    // JSON-RPC to avoid re-testing rmcp client code that other suites
    // already cover.
    let dir = tempfile::tempdir().unwrap();
    let mut child = Command::new(bin())
        .arg("--dir")
        .arg(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    let init = serde_json::json!({
        "jsonrpc":"2.0","id":1,"method":"initialize",
        "params": {
            "protocolVersion":"2025-06-18",
            "capabilities":{},
            "clientInfo":{"name":"cli-test","version":"0"}
        }
    });
    stdin
        .write_all(format!("{init}\n").as_bytes())
        .await
        .unwrap();
    let mut line = String::new();
    timeout(Duration::from_secs(10), reader.read_line(&mut line))
        .await
        .unwrap()
        .unwrap();

    let notif = serde_json::json!({ "jsonrpc":"2.0", "method":"notifications/initialized" });
    stdin
        .write_all(format!("{notif}\n").as_bytes())
        .await
        .unwrap();

    let call = serde_json::json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params": {
            "name":"devlog_new",
            "arguments":{ "section":"dir-test","text":"wrote here" }
        }
    });
    stdin
        .write_all(format!("{call}\n").as_bytes())
        .await
        .unwrap();

    let mut resp = String::new();
    timeout(Duration::from_secs(10), reader.read_line(&mut resp))
        .await
        .unwrap()
        .unwrap();
    drop(stdin);

    let _ = timeout(Duration::from_secs(5), child.wait()).await;
    let _ = child.kill().await;

    // Must have landed in the supplied --dir.
    let file = dir
        .path()
        .join("DEVLOG")
        .join("dir-test")
        .join("dir-test-devlog.md");
    assert!(
        file.is_file(),
        "--dir was not respected: expected {} to exist",
        file.display()
    );
}
