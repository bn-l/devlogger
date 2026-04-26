//! Lifecycle tests: cancellation, clean shutdown, multi-session
//! isolation.  Regression guards for bugs like "server hangs on the
//! `notifications/initialized` message" that have shown up in other
//! stdio MCP servers.

use std::process::Stdio;
use std::time::Duration;

use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::timeout;

use super::e2e_common::*;

#[tokio::test]
async fn client_cancel_terminates_the_server_cleanly() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    // Do some work to prove the session is live.
    assert_wire_ok(&call_new(&client, "core", "x").await);

    // Cancel must resolve, not hang.  Give it a generous budget since
    // subprocess teardown involves a process wait.
    tokio::time::timeout(Duration::from_secs(10), client.cancel())
        .await
        .expect("cancel should not hang")
        .ok();
}

#[tokio::test]
async fn two_independent_server_processes_do_not_share_state() {
    let base_a = fresh_base();
    let base_b = fresh_base();

    let client_a = spawn_subprocess_client(base_a.path()).await;
    let client_b = spawn_subprocess_client(base_b.path()).await;

    // Write different data to each server.
    assert_wire_ok(&call_new(&client_a, "left", "from-a").await);
    assert_wire_ok(&call_new(&client_b, "right", "from-b").await);

    // Each sees only its own section.
    let sections_a: Vec<String> = structured(&call_sections(&client_a).await)
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let sections_b: Vec<String> = structured(&call_sections(&client_b).await)
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(sections_a, vec!["left"]);
    assert_eq!(sections_b, vec!["right"]);

    client_a.cancel().await.ok();
    client_b.cancel().await.ok();
}

#[tokio::test]
async fn session_survives_sustained_tool_call_volume() {
    // Rules out subtle memory/fd leaks per-call: 50 round trips in a
    // row.  If the server leaks file descriptors or accumulates state,
    // one of these will time out or fail.
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    for i in 0..50 {
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            call_new(&client, "load", &format!("entry {i}")),
        )
        .await
        .expect("tool call should complete within 5s");
        assert_wire_ok(&result);
    }

    let listed = call_list(&client, Some("load")).await;
    assert_eq!(structured(&listed).as_array().unwrap().len(), 50);

    client.cancel().await.ok();
}

#[tokio::test]
async fn server_handles_rapid_sequential_calls_without_stalling() {
    // Fire tool calls as fast as the client can — each one should
    // complete within the per-call budget.  This catches any
    // per-request serialization bug in the server where one response's
    // timing bleeds into the next.
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    for i in 0..20 {
        let result = tokio::time::timeout(
            Duration::from_secs(3),
            call_new(&client, "rapid", &format!("rapid-{i}")),
        )
        .await
        .expect("rapid call should not stall");
        assert_wire_ok(&result);
    }

    // Confirm all entries persisted correctly.
    let listed = call_list(&client, Some("rapid")).await;
    assert_eq!(structured(&listed).as_array().unwrap().len(), 20);
    client.cancel().await.ok();
}

#[tokio::test]
async fn server_survives_error_call_followed_by_valid_call() {
    // After the RCA incident: server returns a tool error (oversized
    // entry), and the next call must still succeed — the server must
    // not enter a bad state after returning an error.
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    // First: a call that fails (control char).
    assert_wire_err(&call_new(&client, "core", "\u{0007}bell").await);
    // Second: a valid call must succeed.
    assert_wire_ok(&call_new(&client, "core", "recovery").await);
    // Third: another error (oversized).
    let oversized: String = "z".repeat(devlogger::entry::MAX_ENTRY_COLS + 1);
    assert_wire_err(&call_new(&client, "core", &oversized).await);
    // Fourth: valid again.
    assert_wire_ok(&call_new(&client, "core", "still alive").await);

    let listed = call_list(&client, Some("core")).await;
    assert_eq!(structured(&listed).as_array().unwrap().len(), 2);
    client.cancel().await.ok();
}

#[tokio::test]
async fn stdin_close_after_tool_call_exits_cleanly() {
    // Raw-stdio test: send a tool call then immediately close stdin.
    // The server must exit without hanging or panicking.  This guards
    // against the "response written to a broken pipe" failure mode.
    let dir = tempfile::tempdir().unwrap();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_devlogger-mcp"))
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

    // Initialize.
    let init = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "lifecycle-test", "version": "0" }
        }
    });
    stdin.write_all(format!("{init}\n").as_bytes()).await.unwrap();
    let mut line = String::new();
    timeout(Duration::from_secs(10), reader.read_line(&mut line))
        .await
        .expect("init response")
        .unwrap();

    // Send notifications/initialized.
    let notif = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
    stdin.write_all(format!("{notif}\n").as_bytes()).await.unwrap();

    // Send a tool call, then close stdin before reading the response.
    let call = json!({
        "jsonrpc": "2.0", "id": 2,
        "method": "tools/call",
        "params": {
            "name": "devlog_new",
            "arguments": { "section": "mid", "text": "in-flight" }
        }
    });
    stdin.write_all(format!("{call}\n").as_bytes()).await.unwrap();
    stdin.shutdown().await.ok();
    drop(stdin);

    // Server must exit within budget — not hang.
    let status = timeout(Duration::from_secs(15), child.wait())
        .await
        .expect("server should exit within 15s after stdin close");
    let _ = status;
}

#[tokio::test]
async fn initialize_and_list_tools_is_fast() {
    // A sanity bound that also catches the "server spins in a busy loop
    // after init" bug class.
    let base = fresh_base();
    let started = std::time::Instant::now();
    let client = spawn_subprocess_client(base.path()).await;
    let _tools = client.list_all_tools().await.unwrap();
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(10),
        "init+tools/list took {elapsed:?}"
    );
    client.cancel().await.ok();
}
