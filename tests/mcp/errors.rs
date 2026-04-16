//! Cross-cutting tests for how the MCP surface handles bad input.
//!
//! The rule: user-input errors — bad section names, bad ids, missing
//! files, multi-line text — must become **tool-level** errors
//! (`CallToolResult::error(...)`), never protocol-level errors.  This
//! lets the LLM see the message and retry with a correction.

use super::common::*;

#[tokio::test]
async fn empty_section_name_is_tool_error_not_protocol() {
    let (server, _dir) = fresh_server();
    let result = server.devlog_new(new_args("", "text")).await.unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn leading_hyphen_section_is_tool_error() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_new(new_args("-bad", "text"))
        .await
        .unwrap();
    assert_err(&result);
}

#[tokio::test]
async fn trailing_hyphen_section_is_tool_error() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_new(new_args("bad-", "text"))
        .await
        .unwrap();
    assert_err(&result);
}

#[tokio::test]
async fn consecutive_hyphens_section_is_tool_error() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_new(new_args("bad--section", "text"))
        .await
        .unwrap();
    assert_err(&result);
}

#[tokio::test]
async fn digits_in_section_name_rejected() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_new(new_args("has1digit", "text"))
        .await
        .unwrap();
    assert_err(&result);
}

#[tokio::test]
async fn underscore_in_section_name_rejected() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_new(new_args("snake_case", "text"))
        .await
        .unwrap();
    assert_err(&result);
}

#[tokio::test]
async fn entry_text_with_cr_rejected() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_new(new_args("core", "bad\rnewline"))
        .await
        .unwrap();
    assert_err(&result);
}

#[tokio::test]
async fn entry_text_with_null_rejected() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_new(new_args("core", "zero\u{0000}byte"))
        .await
        .unwrap();
    assert_err(&result);
}

#[tokio::test]
async fn tab_in_entry_text_is_allowed() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_new(new_args("core", "tabs\tare\tfine"))
        .await
        .unwrap();
    assert_ok(&result);
}

#[tokio::test]
async fn read_missing_file_is_tool_error_not_protocol() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_read(read_args("nothing", None))
        .await
        .expect("must not be a protocol error");
    assert_err(&result);
}

#[tokio::test]
async fn update_missing_file_is_tool_error_not_protocol() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_update(update_args("nothing", "1", "text"))
        .await
        .expect("must not be a protocol error");
    assert_err(&result);
}

#[tokio::test]
async fn list_missing_section_is_tool_error_not_protocol() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_list(list_args(Some("nothing")))
        .await
        .expect("must not be a protocol error");
    assert_err(&result);
}
