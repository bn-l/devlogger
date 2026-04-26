//! Shared helpers for the wire-level end-to-end suite.  Two transports
//! are exposed:
//!
//! - [`spawn_subprocess_client`] boots the real `devlogger-mcp` binary
//!   over `TokioChildProcess` — same code path a host like Claude
//!   Desktop would take.
//! - [`spawn_inprocess_client`] runs [`DevlogServer`] as a tokio task
//!   behind a `tokio::io::duplex` pipe — this is the pattern rmcp uses
//!   in its own internal tests (see the rust-sdk's
//!   `crates/rmcp/tests/test_notification.rs`).  Much faster, no fork.

#![allow(dead_code)]

use std::path::Path;

use rmcp::model::{CallToolRequestParams, CallToolResult, ClientInfo, Implementation};
use rmcp::service::{RoleClient, RunningService};
use rmcp::transport::TokioChildProcess;
use rmcp::{ServiceExt, object};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::process::Command;

use devlogger::mcp::DevlogServer;

/// Minimal `ClientInfo` — rmcp's client-side initialize packet.  Default
/// is fine, just stamp a name so the server-side logs are readable.
pub fn client_info() -> ClientInfo {
    let mut info = ClientInfo::default();
    info.client_info = {
        let mut imp = Implementation::default();
        imp.name = "devlogger-e2e-tests".into();
        imp.version = "0.0.0".into();
        imp
    };
    info
}

/// Boot the real `devlogger-mcp` binary pointed at `base` and return a
/// connected rmcp client plus the `TempDir` that must outlive it.
pub async fn spawn_subprocess_client(
    base: &Path,
) -> RunningService<RoleClient, ClientInfo> {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_devlogger-mcp"));
    cmd.arg("--dir").arg(base);
    let transport = TokioChildProcess::new(cmd).expect("spawn devlogger-mcp");
    client_info()
        .serve(transport)
        .await
        .expect("initialize handshake failed")
}

/// Same as [`spawn_subprocess_client`] but with additional env vars set
/// on the child process.
pub async fn spawn_subprocess_client_with_env(
    base: &Path,
    env: &[(&str, &str)],
) -> RunningService<RoleClient, ClientInfo> {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_devlogger-mcp"));
    cmd.arg("--dir").arg(base);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let transport = TokioChildProcess::new(cmd).expect("spawn devlogger-mcp");
    client_info()
        .serve(transport)
        .await
        .expect("initialize handshake failed")
}

/// Spin up a `DevlogServer` inside the current process using a duplex
/// pipe transport (rmcp's idiomatic in-test pattern).  Skips the fork
/// cost but still exercises the full rmcp protocol layer: initialize
/// handshake, tools/list, tools/call, JSON serialization of schemas and
/// results.
pub async fn spawn_inprocess_client(
    base: &Path,
) -> RunningService<RoleClient, ClientInfo> {
    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    let base = base.to_path_buf();
    tokio::spawn(async move {
        let server = DevlogServer::new(base)
            .serve(server_io)
            .await
            .expect("in-process server init");
        let _ = server.waiting().await;
    });
    client_info()
        .serve(client_io)
        .await
        .expect("in-process initialize handshake failed")
}

/// Call a tool with JSON arguments, returning the full `CallToolResult`.
/// Protocol errors panic — tests expecting a tool-level error should
/// check `result.is_error`.
pub async fn call(
    client: &RunningService<RoleClient, ClientInfo>,
    name: &'static str,
    arguments: Value,
) -> CallToolResult {
    let args_obj = arguments
        .as_object()
        .expect("arguments must be an object")
        .clone();
    client
        .call_tool(CallToolRequestParams::new(name).with_arguments(args_obj))
        .await
        .expect("tools/call protocol error")
}

/// Call `devlog_new` with no `base_dir` override.
pub async fn call_new(
    client: &RunningService<RoleClient, ClientInfo>,
    section: &str,
    text: &str,
) -> CallToolResult {
    call(
        client,
        "devlog_new",
        json!({ "section": section, "text": text }),
    )
    .await
}

pub async fn call_sections(
    client: &RunningService<RoleClient, ClientInfo>,
) -> CallToolResult {
    call(client, "devlog_sections", json!({})).await
}

pub async fn call_list(
    client: &RunningService<RoleClient, ClientInfo>,
    section: Option<&str>,
) -> CallToolResult {
    let args = match section {
        Some(s) => object!({ "section": s }).into(),
        None => json!({}),
    };
    call(client, "devlog_list", args).await
}

pub async fn call_read(
    client: &RunningService<RoleClient, ClientInfo>,
    section: &str,
    n: Option<usize>,
) -> CallToolResult {
    let args = match n {
        Some(n) => json!({ "section": section, "n": n }),
        None => json!({ "section": section }),
    };
    call(client, "devlog_read", args).await
}

pub async fn call_update(
    client: &RunningService<RoleClient, ClientInfo>,
    section: &str,
    id: &str,
    text: &str,
) -> CallToolResult {
    call(
        client,
        "devlog_update",
        json!({ "section": section, "id": id, "text": text }),
    )
    .await
}

pub async fn call_move(
    client: &RunningService<RoleClient, ClientInfo>,
    from: &str,
    id: &str,
    to: &str,
) -> CallToolResult {
    call(
        client,
        "devlog_move",
        json!({ "from_section": from, "id": id, "to_section": to }),
    )
    .await
}

/// Fresh `TempDir` used as the server's `--dir`.
pub fn fresh_base() -> TempDir {
    tempfile::tempdir().expect("tempdir")
}

/// Concatenate all text-type content entries into a single String.
pub fn text_content(result: &CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| match &c.raw {
            rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

pub fn structured(result: &CallToolResult) -> &Value {
    result
        .structured_content
        .as_ref()
        .expect("expected structured_content")
}

pub fn is_tool_error(result: &CallToolResult) -> bool {
    matches!(result.is_error, Some(true))
}

pub fn assert_wire_ok(result: &CallToolResult) {
    if is_tool_error(result) {
        panic!(
            "expected success, got wire tool error: {}",
            text_content(result)
        );
    }
}

pub fn assert_wire_err(result: &CallToolResult) -> String {
    assert!(
        is_tool_error(result),
        "expected tool error, got success: {}",
        text_content(result)
    );
    text_content(result)
}

/// On-disk path of a section's devlog.
pub fn section_file(base: &Path, section: &str) -> std::path::PathBuf {
    base.join("DEVLOG")
        .join(section)
        .join(format!("{section}-devlog.md"))
}
