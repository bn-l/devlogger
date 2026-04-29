//! Stdout cleanliness — the critical stdio-MCP invariant.  Any byte on
//! stdout that isn't a valid JSON-RPC 2.0 frame immediately corrupts the
//! protocol stream and crashes the host.  A stray `println!`, a Rust
//! panic message, or an error trace leaking from `tracing_subscriber`
//! are all instant fails.
//!
//! These tests bypass the rmcp client entirely and speak raw NDJSON
//! JSON-RPC on stdin/stdout, then validate every line on stdout parses
//! and has the expected structure.  The binary is spawned with
//! `RUST_LOG=trace` so any logging regressions are caught.

use std::process::Stdio;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

async fn read_line(reader: &mut BufReader<tokio::process::ChildStdout>) -> String {
    let mut buf = String::new();
    timeout(Duration::from_secs(10), reader.read_line(&mut buf))
        .await
        .expect("stdout line should arrive within 10s")
        .expect("read_line");
    buf
}

#[tokio::test]
async fn every_line_on_stdout_is_a_valid_jsonrpc_frame_even_with_rust_log_trace() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_devlogger-mcp"))
        .arg("--dir")
        .arg(dir.path())
        .env("RUST_LOG", "trace")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // 1. initialize request
    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "cleanliness-test", "version": "0" }
        }
    });
    stdin
        .write_all(format!("{init}\n").as_bytes())
        .await
        .unwrap();

    let line1 = read_line(&mut reader).await;
    let resp1: Value =
        serde_json::from_str(line1.trim_end()).expect("initialize response must be valid JSON");
    assert_eq!(resp1["jsonrpc"], "2.0");
    assert_eq!(resp1["id"], 1);

    // 2. initialized notification
    let notif = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
    stdin
        .write_all(format!("{notif}\n").as_bytes())
        .await
        .unwrap();

    // 3. tools/list
    let list = json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" });
    stdin
        .write_all(format!("{list}\n").as_bytes())
        .await
        .unwrap();

    let line2 = read_line(&mut reader).await;
    let resp2: Value =
        serde_json::from_str(line2.trim_end()).expect("tools/list response must be valid JSON");
    assert_eq!(resp2["jsonrpc"], "2.0");
    assert_eq!(resp2["id"], 2);
    let tools = resp2["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 6);

    // 4. tools/call devlog_new
    let call = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "devlog_new",
            "arguments": { "section": "wire", "text": "clean bytes only" }
        }
    });
    stdin
        .write_all(format!("{call}\n").as_bytes())
        .await
        .unwrap();

    let line3 = read_line(&mut reader).await;
    let resp3: Value =
        serde_json::from_str(line3.trim_end()).expect("tools/call response must be valid JSON");
    assert_eq!(resp3["id"], 3);
    assert!(
        resp3["result"]["isError"].as_bool() != Some(true),
        "unexpected tool error"
    );

    stdin.shutdown().await.ok();
    drop(stdin);

    // Give the server a moment to flush and exit; then assert no stray
    // bytes were left on stdout.
    let _ = child.kill().await;
    let output = child.wait_with_output().await.unwrap();
    let leftover = String::from_utf8_lossy(&output.stdout);
    for (idx, line) in leftover.lines().enumerate() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        serde_json::from_str::<Value>(trimmed).unwrap_or_else(|e| {
            panic!("stdout leftover line {idx} is not valid JSON-RPC: {e}\nline: {trimmed:?}")
        });
    }

    // stderr is allowed to contain log output under RUST_LOG=trace;
    // just confirm it didn't crash.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked"), "server panicked: {stderr}");
}

#[tokio::test]
async fn startup_log_goes_to_stderr_not_stdout_when_enabled() {
    // The startup banner is gated at `debug` — only visible when the
    // user explicitly opts in via `RUST_LOG`.  When it IS emitted it
    // must still go to stderr (never stdout, which is the JSON-RPC
    // channel).  A sibling test asserts the default is fully silent.
    let dir = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_devlogger-mcp"))
        .arg("--dir")
        .arg(dir.path())
        .env("RUST_LOG", "devlogger_mcp=debug")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    let mut stdin = child.stdin.take().unwrap();
    let stdout_pipe = child.stdout.take().unwrap();
    let mut stderr_pipe = child.stderr.take().unwrap();
    let mut stdout_reader = BufReader::new(stdout_pipe);

    // Send an initialize and read its response.  The round-trip proves
    // the server reached `serve(stdio())` — strictly after the
    // `tracing::info!("devlogger-mcp starting")` call in main().
    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "cleanliness-test", "version": "0" }
        }
    });
    stdin
        .write_all(format!("{init}\n").as_bytes())
        .await
        .unwrap();
    let init_resp = read_line(&mut stdout_reader).await;

    // Graceful shutdown: dropping stdin closes the server's input.
    drop(stdin);
    let _ = timeout(Duration::from_secs(5), child.wait()).await;
    let _ = child.kill().await;

    // Drain both streams to EOF.
    let mut stdout_rest = String::new();
    let _ = timeout(
        Duration::from_secs(2),
        stdout_reader.read_to_string(&mut stdout_rest),
    )
    .await;
    let mut stderr_all = String::new();
    let _ = timeout(
        Duration::from_secs(2),
        stderr_pipe.read_to_string(&mut stderr_all),
    )
    .await;
    let stdout = format!("{init_resp}{stdout_rest}");

    // The "devlogger-mcp starting" info log MUST be on stderr, NOT stdout.
    assert!(
        !stdout.contains("devlogger-mcp starting"),
        "log line leaked to stdout: {stdout}"
    );
    assert!(
        stderr_all.contains("devlogger-mcp starting"),
        "startup log missing from stderr: {stderr_all}"
    );

    // Every stdout line must still parse as JSON-RPC.
    for (idx, line) in stdout.lines().enumerate() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        serde_json::from_str::<Value>(trimmed)
            .unwrap_or_else(|e| panic!("stdout line {idx} not valid JSON-RPC: {e}: {trimmed:?}"));
    }
}

#[tokio::test]
async fn default_startup_has_no_banner_on_stderr() {
    // Claude Code 2.1.x logs every stderr byte as a "Server stderr"
    // line in the MCP connection log.  An INFO-level banner on each
    // launch was polluting those logs and making real errors harder
    // to spot, so the banner now lives behind `RUST_LOG=debug`.
    //
    // Note: the claude-code-race workaround (see
    // `src/mcp/claude_code_race_workaround.rs`) intentionally emits
    // a `warn!` on every initialize — that IS allowed to appear, and
    // appearing is the point: it self-advertises so the workaround
    // can't be quietly forgotten.  This test just guards the generic
    // startup banner.
    let dir = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_devlogger-mcp"))
        .arg("--dir")
        .arg(dir.path())
        .env_remove("RUST_LOG")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    let mut stdin = child.stdin.take().unwrap();
    let stdout_pipe = child.stdout.take().unwrap();
    let mut stderr_pipe = child.stderr.take().unwrap();
    let mut stdout_reader = BufReader::new(stdout_pipe);

    // Drive a full initialize round-trip so we know the server
    // reached `serve(stdio())` — any startup log would already have
    // been emitted by this point.
    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "silence-test", "version": "0" }
        }
    });
    stdin
        .write_all(format!("{init}\n").as_bytes())
        .await
        .unwrap();
    let _ = read_line(&mut stdout_reader).await;

    drop(stdin);
    let _ = timeout(Duration::from_secs(5), child.wait()).await;
    let _ = child.kill().await;

    let mut stderr_all = String::new();
    let _ = timeout(
        Duration::from_secs(2),
        stderr_pipe.read_to_string(&mut stderr_all),
    )
    .await;

    assert!(
        !stderr_all.contains("devlogger-mcp starting"),
        "startup banner leaked to stderr on a default launch: {stderr_all:?}"
    );
}

#[tokio::test]
async fn server_exits_cleanly_when_stdin_closes() {
    // Closing stdin without a prior shutdown message should cause a
    // graceful exit — not a hang, not a panic.  Catches the common
    // "server loops forever on EOF" bug.
    let dir = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_devlogger-mcp"))
        .arg("--dir")
        .arg(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    // Close stdin immediately.
    drop(child.stdin.take().unwrap());

    let status = timeout(Duration::from_secs(15), child.wait())
        .await
        .expect("server should exit within 15s when stdin closes");
    // Any exit status is acceptable — we only care that it terminated.
    let _ = status;
}
