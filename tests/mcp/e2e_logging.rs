//! Adversarial tests for the central server-side JSONL logging.
//!
//! The logging must be best-effort: never crash the server, never
//! corrupt stdout (the JSON-RPC channel), and actually produce usable
//! log records when the log directory is writable.

use std::process::Stdio;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::timeout;

use std::sync::Arc;

use super::e2e_common::*;

/// Helper: read one JSON-RPC response line from the server.
async fn read_response(reader: &mut BufReader<tokio::process::ChildStdout>) -> Value {
    let mut buf = String::new();
    timeout(Duration::from_secs(10), reader.read_line(&mut buf))
        .await
        .expect("response should arrive within 10s")
        .expect("read_line");
    serde_json::from_str(buf.trim_end()).expect("response must be valid JSON")
}

/// Perform a full initialize handshake and send notifications/initialized.
/// Returns after the handshake is complete.
async fn do_handshake(
    stdin: &mut tokio::process::ChildStdin,
    reader: &mut BufReader<tokio::process::ChildStdout>,
) {
    let init = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "log-test", "version": "0" }
        }
    });
    stdin
        .write_all(format!("{init}\n").as_bytes())
        .await
        .unwrap();
    let resp = read_response(reader).await;
    assert_eq!(resp["id"], 1);

    let notif = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
    stdin
        .write_all(format!("{notif}\n").as_bytes())
        .await
        .unwrap();
}

/// Send a tools/call and read the response.
async fn do_tool_call(
    stdin: &mut tokio::process::ChildStdin,
    reader: &mut BufReader<tokio::process::ChildStdout>,
    id: u64,
    tool: &str,
    arguments: Value,
) -> Value {
    let call = json!({
        "jsonrpc": "2.0", "id": id,
        "method": "tools/call",
        "params": { "name": tool, "arguments": arguments }
    });
    stdin
        .write_all(format!("{call}\n").as_bytes())
        .await
        .unwrap();
    read_response(reader).await
}

#[tokio::test]
async fn server_does_not_crash_when_log_dir_is_unwritable() {
    // DEVLOGGER_LOG_DIR pointed at a nonexistent path under /dev/null.
    // The server must start, handle tool calls, and exit cleanly.
    let dir = tempfile::tempdir().unwrap();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_devlogger-mcp"))
        .arg("--dir")
        .arg(dir.path())
        .env("DEVLOGGER_LOG_DIR", "/dev/null/impossible/path")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    do_handshake(&mut stdin, &mut reader).await;

    // Tool calls must still work.
    let resp = do_tool_call(
        &mut stdin,
        &mut reader,
        2,
        "devlog_new",
        json!({"section": "log", "text": "unwritable log dir"}),
    )
    .await;
    assert!(resp["result"]["isError"].is_null() || resp["result"]["isError"] == false);

    stdin.shutdown().await.ok();
    drop(stdin);
    let status = timeout(Duration::from_secs(10), child.wait())
        .await
        .expect("server should exit")
        .expect("wait");
    // Server must not have crashed.
    assert!(
        status.success(),
        "server exited with failure when log dir was unwritable: {status}"
    );
}

#[tokio::test]
async fn server_does_not_crash_when_home_is_unset() {
    let dir = tempfile::tempdir().unwrap();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_devlogger-mcp"))
        .arg("--dir")
        .arg(dir.path())
        .env_remove("HOME")
        .env_remove("DEVLOGGER_LOG_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    do_handshake(&mut stdin, &mut reader).await;

    let resp = do_tool_call(
        &mut stdin,
        &mut reader,
        2,
        "devlog_new",
        json!({"section": "log", "text": "no home"}),
    )
    .await;
    assert!(resp["result"]["isError"].is_null() || resp["result"]["isError"] == false);

    stdin.shutdown().await.ok();
    drop(stdin);
    let status = timeout(Duration::from_secs(10), child.wait())
        .await
        .expect("server should exit")
        .expect("wait");
    assert!(status.success(), "server crashed with no HOME: {status}");
}

#[tokio::test]
async fn log_file_is_created_at_devlogger_log_dir() {
    let log_dir = tempfile::tempdir().unwrap();
    let base_dir = tempfile::tempdir().unwrap();
    let client = spawn_subprocess_client_with_env(
        base_dir.path(),
        &[("DEVLOGGER_LOG_DIR", log_dir.path().to_str().unwrap())],
    )
    .await;

    // Do some work.
    assert_wire_ok(&call_new(&client, "log", "test entry").await);
    assert_wire_err(&call_new(&client, "log", "\u{0007}bad").await);

    client.cancel().await.ok();

    // Check log files were created.
    let entries: Vec<_> = std::fs::read_dir(log_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "expected exactly one log file, got: {entries:?}"
    );

    let log_path = entries[0].path();
    let log_name = log_path.file_name().unwrap().to_string_lossy();
    assert!(
        log_name.starts_with("mcp-server.") && log_name.ends_with(".jsonl"),
        "unexpected log filename: {log_name}"
    );

    // Every line must be valid JSON.
    let contents = std::fs::read_to_string(&log_path).unwrap();
    assert!(!contents.is_empty(), "log file is empty");
    for (i, line) in contents.lines().enumerate() {
        let parsed: Value = serde_json::from_str(line).unwrap_or_else(|e| {
            panic!("log line {i} is not valid JSON: {e}\nline: {line}");
        });
        // Must have standard tracing fields.
        assert!(
            parsed.get("timestamp").is_some(),
            "line {i} missing timestamp"
        );
        assert!(parsed.get("level").is_some(), "line {i} missing level");
    }
}

#[tokio::test]
async fn log_file_contains_tool_completion_records() {
    let log_dir = tempfile::tempdir().unwrap();
    let base_dir = tempfile::tempdir().unwrap();
    let client = spawn_subprocess_client_with_env(
        base_dir.path(),
        &[("DEVLOGGER_LOG_DIR", log_dir.path().to_str().unwrap())],
    )
    .await;

    // Success call.
    assert_wire_ok(&call_new(&client, "core", "good entry").await);
    // Error call (oversized).
    let oversized: String = "x".repeat(devlogger::entry::MAX_ENTRY_COLS + 1);
    assert_wire_err(&call_new(&client, "core", &oversized).await);
    // Another success.
    assert_wire_ok(&call_sections(&client).await);

    client.cancel().await.ok();

    // Read all log lines.
    let entries: Vec<_> = std::fs::read_dir(log_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    let log_path = entries[0].path();
    let contents = std::fs::read_to_string(&log_path).unwrap();
    let tool_lines: Vec<Value> = contents
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| {
            v.get("fields")
                .and_then(|f| f.get("message"))
                .and_then(|m| m.as_str())
                == Some("tool call completed")
        })
        .collect();

    // Must have at least 3 tool completion records.
    assert!(
        tool_lines.len() >= 3,
        "expected >= 3 tool completion records, got {}: {tool_lines:#?}",
        tool_lines.len()
    );

    // Check the success record has the right fields.
    let success_line = tool_lines.iter().find(|v| {
        v["fields"]["result"].as_str() == Some("success")
            && v["fields"]["tool"].as_str() == Some("devlog_new")
    });
    assert!(success_line.is_some(), "no success record for devlog_new");
    let s = success_line.unwrap();
    assert!(s["fields"]["elapsed_ms"].is_number(), "missing elapsed_ms");
    assert!(s["fields"]["pid"].is_number(), "missing pid");

    // Check the error record.
    let error_line = tool_lines
        .iter()
        .find(|v| v["fields"]["result"].as_str() == Some("tool_error"));
    assert!(error_line.is_some(), "no tool_error record found");
}

#[tokio::test]
async fn log_file_does_not_contain_entry_text() {
    // Privacy: the log must not contain the actual entry text.
    let log_dir = tempfile::tempdir().unwrap();
    let base_dir = tempfile::tempdir().unwrap();
    let client = spawn_subprocess_client_with_env(
        base_dir.path(),
        &[("DEVLOGGER_LOG_DIR", log_dir.path().to_str().unwrap())],
    )
    .await;

    let secret = "super-secret-entry-text-that-must-not-leak";
    assert_wire_ok(&call_new(&client, "core", secret).await);

    client.cancel().await.ok();

    let entries: Vec<_> = std::fs::read_dir(log_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    let contents = std::fs::read_to_string(entries[0].path()).unwrap();
    assert!(
        !contents.contains(secret),
        "log file contains entry text — privacy violation"
    );
}

#[tokio::test]
async fn stdout_stays_clean_when_log_dir_is_unwritable() {
    // Even when logging fails, stdout must contain only valid JSON-RPC.
    let dir = tempfile::tempdir().unwrap();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_devlogger-mcp"))
        .arg("--dir")
        .arg(dir.path())
        .env("DEVLOGGER_LOG_DIR", "/dev/null/impossible")
        .env("RUST_LOG", "trace")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    do_handshake(&mut stdin, &mut reader).await;

    let resp = do_tool_call(
        &mut stdin,
        &mut reader,
        2,
        "devlog_new",
        json!({"section": "x", "text": "y"}),
    )
    .await;
    assert_eq!(resp["id"], 2);

    stdin.shutdown().await.ok();
    drop(stdin);
    let _ = child.kill().await;
    let output = child.wait_with_output().await.unwrap();
    let leftover = String::from_utf8_lossy(&output.stdout);
    for (idx, line) in leftover.lines().enumerate() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        serde_json::from_str::<Value>(trimmed).unwrap_or_else(|e| {
            panic!("stdout line {idx} is not valid JSON-RPC: {e}\nline: {trimmed:?}")
        });
    }
    // stderr must not contain panics.
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked"), "server panicked: {stderr}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_tool_calls_produce_intact_log_lines() {
    // Hammer the server with concurrent tool calls (mix of success and
    // error).  Every log line must be valid JSON — no interleaved
    // partial writes, no corruption from Mutex contention.
    let log_dir = tempfile::tempdir().unwrap();
    let base_dir = tempfile::tempdir().unwrap();
    let client = spawn_subprocess_client_with_env(
        base_dir.path(),
        &[("DEVLOGGER_LOG_DIR", log_dir.path().to_str().unwrap())],
    )
    .await;
    let client = Arc::new(client);

    const N: usize = 100;
    let mut handles = Vec::with_capacity(N);

    for i in 0..N {
        let c = client.clone();
        handles.push(tokio::spawn(async move {
            if i % 5 == 0 {
                // Every 5th call is an error (oversized entry).
                let big = "x".repeat(devlogger::entry::MAX_ENTRY_COLS + 1);
                call_new(&c, "stress", &big).await
            } else if i % 7 == 0 {
                // Some list calls mixed in.
                call_sections(&c).await
            } else {
                call_new(&c, "stress", &format!("concurrent-{i}")).await
            }
        }));
    }

    for h in handles {
        let _ = h.await.unwrap(); // don't care about individual results
    }

    Arc::into_inner(client).unwrap().cancel().await.ok();

    // Read the log and validate every single line.
    let entries: Vec<_> = std::fs::read_dir(log_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(!entries.is_empty(), "no log file created");

    let contents = std::fs::read_to_string(entries[0].path()).unwrap();
    let lines: Vec<&str> = contents.lines().collect();

    // Must have at least N tool completion records (plus the
    // initialize workaround warning).
    let mut tool_completions = 0;
    for (i, line) in lines.iter().enumerate() {
        let parsed: Value = serde_json::from_str(line).unwrap_or_else(|e| {
            // This is the critical assertion: a corrupted line means
            // the Mutex<File> writes were not atomic.
            panic!(
                "log line {i} is CORRUPTED (not valid JSON): {e}\n\
                 line: {line}\n\
                 This means concurrent writes interleaved."
            );
        });
        if parsed["fields"]["message"].as_str() == Some("tool call completed") {
            tool_completions += 1;
        }
    }
    assert!(
        tool_completions >= N,
        "expected >= {N} tool completion records, got {tool_completions} \
         (out of {} total log lines)",
        lines.len()
    );

    // Verify we got both success and tool_error records.
    let successes = contents.matches("\"result\":\"success\"").count();
    let errors = contents.matches("\"result\":\"tool_error\"").count();
    assert!(successes > 0, "no success records in concurrent test");
    assert!(errors > 0, "no tool_error records in concurrent test");
}
