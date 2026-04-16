//! Shared helpers for the MCP server test suite.
//!
//! Each test gets its own `tempfile::TempDir` as the server's default
//! base, so the `DEVLOG/` tree lives in isolation.  Most tests call
//! [`DevlogServer`]'s tool methods directly — that gives us fine-grained
//! assertions on the `CallToolResult` structure without needing a live
//! transport.  The protocol-layer test (`protocol_stdio.rs`) spawns the
//! real `devlogger-mcp` binary and speaks JSON-RPC on its stdio.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, RawContent};
use serde_json::Value;
use tempfile::TempDir;

use devlogger::mcp::DevlogServer;
use devlogger::mcp::args::{ListArgs, NewArgs, ReadArgs, SectionsArgs, UpdateArgs};

/// Build a server rooted at a fresh `TempDir`.  The returned `TempDir`
/// must be kept alive for the lifetime of the test (dropping it deletes
/// the backing directory).
pub fn fresh_server() -> (DevlogServer, TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = DevlogServer::new(dir.path().to_path_buf());
    (server, dir)
}

/// Build a server rooted at a specific path (not a tempdir).  Used by
/// `base_dir.rs` to verify override semantics.
pub fn server_at(path: &Path) -> DevlogServer {
    DevlogServer::new(path.to_path_buf())
}

/// Construct a `Parameters<NewArgs>` with no `base_dir` override.
pub fn new_args(section: &str, text: &str) -> Parameters<NewArgs> {
    Parameters(NewArgs {
        section: section.into(),
        text: text.into(),
        base_dir: None,
    })
}

pub fn new_args_at(section: &str, text: &str, base_dir: &Path) -> Parameters<NewArgs> {
    Parameters(NewArgs {
        section: section.into(),
        text: text.into(),
        base_dir: Some(base_dir.to_string_lossy().into_owned()),
    })
}

pub fn list_args(section: Option<&str>) -> Parameters<ListArgs> {
    Parameters(ListArgs {
        section: section.map(str::to_string),
        base_dir: None,
    })
}

pub fn list_args_at(section: Option<&str>, base_dir: &Path) -> Parameters<ListArgs> {
    Parameters(ListArgs {
        section: section.map(str::to_string),
        base_dir: Some(base_dir.to_string_lossy().into_owned()),
    })
}

pub fn sections_args() -> Parameters<SectionsArgs> {
    Parameters(SectionsArgs { base_dir: None })
}

pub fn sections_args_at(base_dir: &Path) -> Parameters<SectionsArgs> {
    Parameters(SectionsArgs {
        base_dir: Some(base_dir.to_string_lossy().into_owned()),
    })
}

pub fn update_args(section: &str, id: &str, text: &str) -> Parameters<UpdateArgs> {
    Parameters(UpdateArgs {
        section: section.into(),
        id: id.into(),
        text: text.into(),
        base_dir: None,
    })
}

pub fn read_args(section: &str, n: Option<usize>) -> Parameters<ReadArgs> {
    Parameters(ReadArgs {
        section: section.into(),
        n,
        base_dir: None,
    })
}

/// Extract the concatenated text from a `CallToolResult`'s content
/// entries.  Non-text entries are skipped.
pub fn text_of(result: &CallToolResult) -> String {
    let mut out = String::new();
    for c in &result.content {
        if let Some(t) = extract_text(c) {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(t);
        }
    }
    out
}

fn extract_text(content: &Content) -> Option<&str> {
    match &content.raw {
        RawContent::Text(t) => Some(&t.text),
        _ => None,
    }
}

/// Extract the structured JSON payload of a `CallToolResult` — panics
/// if absent (tests that use this expect a structured result).
pub fn structured(result: &CallToolResult) -> &Value {
    result
        .structured_content
        .as_ref()
        .expect("expected structured_content on result")
}

/// `true` when the result represents a tool-level error (is_error == Some(true)).
pub fn is_error(result: &CallToolResult) -> bool {
    matches!(result.is_error, Some(true))
}

/// Convenience: extract the `text` field from a JSON object (panic
/// otherwise), e.g. for structured entry payloads.
pub fn json_text(v: &Value) -> &str {
    v.get("text")
        .and_then(Value::as_str)
        .expect("expected object with string `text` field")
}

/// Assert that a `CallToolResult` is *not* a tool error, pretty-printing
/// the message payload when the assertion fails.
pub fn assert_ok(result: &CallToolResult) {
    if is_error(result) {
        panic!("expected success, got tool error: {}", text_of(result));
    }
}

/// Assert that a `CallToolResult` *is* a tool error, and return the
/// textual payload (for substring checks).
pub fn assert_err(result: &CallToolResult) -> String {
    assert!(
        is_error(result),
        "expected tool error, got success: {}",
        text_of(result)
    );
    text_of(result)
}

/// The canonical on-disk devlog path for a section under `base`.
pub fn section_file(base: &Path, section: &str) -> PathBuf {
    base.join("DEVLOG")
        .join(section)
        .join(format!("{section}-devlog.md"))
}

/// Full path to the compiled `devlogger-mcp` binary, provided by Cargo.
pub fn mcp_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_devlogger-mcp"))
}
