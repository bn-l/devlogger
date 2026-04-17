//! Wire-level error-path coverage.  Every user-input error must arrive
//! at the client as a `CallToolResult` with `is_error == true` and a
//! text message — never as a JSON-RPC protocol error.  An LLM looking at
//! a protocol error cannot retry gracefully; a tool error it can read
//! and adapt to.

use serde_json::json;

use super::e2e_common::*;

#[tokio::test]
async fn new_with_uppercase_section_is_tool_error_over_wire() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    let result = call_new(&client, "BadCase", "text").await;
    let msg = assert_wire_err(&result);
    assert!(
        msg.contains("illegal character") || msg.contains("invalid section"),
        "unexpected message: {msg}"
    );
    client.cancel().await.ok();
}

#[tokio::test]
async fn new_with_digits_in_section_is_tool_error_over_wire() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    assert_wire_err(&call_new(&client, "has1digit", "text").await);
    client.cancel().await.ok();
}

#[tokio::test]
async fn new_with_empty_section_is_tool_error_over_wire() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    assert_wire_err(&call_new(&client, "", "text").await);
    client.cancel().await.ok();
}

#[tokio::test]
async fn new_with_newline_in_text_is_tool_error_over_wire() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    let msg = assert_wire_err(&call_new(&client, "core", "a\nb").await);
    assert!(msg.contains("newline"), "{msg}");
    client.cancel().await.ok();
}

#[tokio::test]
async fn new_with_control_character_in_text_is_tool_error_over_wire() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    let result = call_new(&client, "core", "\u{0007}bell").await;
    let msg = assert_wire_err(&result);
    assert!(msg.contains("control") || msg.contains("newline"), "{msg}");
    client.cancel().await.ok();
}

#[tokio::test]
async fn list_missing_section_is_tool_error_over_wire() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    let result = call_list(&client, Some("ghost")).await;
    let msg = assert_wire_err(&result);
    assert!(msg.contains("not found"), "{msg}");
    client.cancel().await.ok();
}

#[tokio::test]
async fn list_invalid_section_name_is_tool_error_over_wire() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    assert_wire_err(&call_list(&client, Some("Bad")).await);
    client.cancel().await.ok();
}

#[tokio::test]
async fn read_missing_section_is_tool_error_over_wire() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    assert_wire_err(&call_read(&client, "gone", None).await);
    client.cancel().await.ok();
}

#[tokio::test]
async fn update_missing_section_is_tool_error_over_wire() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    assert_wire_err(&call_update(&client, "never", "1", "text").await);
    client.cancel().await.ok();
}

#[tokio::test]
async fn update_unknown_id_is_tool_error_over_wire() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    assert_wire_ok(&call_new(&client, "core", "x").await);
    let msg = assert_wire_err(&call_update(&client, "core", "9999", "text").await);
    assert!(msg.contains("no entry"), "{msg}");
    client.cancel().await.ok();
}

#[tokio::test]
async fn update_rejects_multiline_replacement_over_wire() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    assert_wire_ok(&call_new(&client, "core", "x").await);
    assert_wire_err(&call_update(&client, "core", "1", "a\nb").await);
    client.cancel().await.ok();
}

#[tokio::test]
async fn move_missing_source_section_is_tool_error_over_wire() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    let msg = assert_wire_err(&call_move(&client, "ghost", "1", "dst").await);
    assert!(msg.contains("not found"), "{msg}");
    client.cancel().await.ok();
}

#[tokio::test]
async fn move_to_same_section_is_tool_error_over_wire() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    assert_wire_ok(&call_new(&client, "core", "only").await);
    let msg = assert_wire_err(&call_move(&client, "core", "1", "core").await);
    assert!(msg.contains("same section"), "{msg}");
    client.cancel().await.ok();
}

#[tokio::test]
async fn move_invalid_dest_section_is_tool_error_over_wire() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    assert_wire_ok(&call_new(&client, "core", "only").await);
    let msg = assert_wire_err(&call_move(&client, "core", "1", "BadCase").await);
    assert!(
        msg.contains("invalid section") || msg.contains("illegal"),
        "{msg}"
    );
    client.cancel().await.ok();
}

#[tokio::test]
async fn missing_required_argument_surfaces_as_protocol_error() {
    // Omitting `text` on `devlog_new` means the JSON payload cannot
    // deserialize into NewArgs — rmcp should reject this at the protocol
    // layer with an error result, not panic the server.
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    let attempt = client
        .call_tool(
            rmcp::model::CallToolRequestParams::new("devlog_new")
                .with_arguments(rmcp::object!({ "section": "missing-text" })),
        )
        .await;
    assert!(
        attempt.is_err(),
        "missing-required-arg must be a protocol error, got {attempt:?}"
    );
    client.cancel().await.ok();
}

#[tokio::test]
async fn unknown_tool_name_surfaces_as_protocol_error() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    let attempt = client
        .call_tool(
            rmcp::model::CallToolRequestParams::new("devlog_explode")
                .with_arguments(rmcp::object!({})),
        )
        .await;
    assert!(
        attempt.is_err(),
        "unknown tool must be a protocol error, got {attempt:?}"
    );
    client.cancel().await.ok();
}

#[tokio::test]
async fn wrong_argument_type_is_rejected_at_protocol_layer() {
    // `section` must be a string; sending a number must not be silently
    // coerced.  rmcp should fail the call.
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    let attempt = client
        .call_tool(
            rmcp::model::CallToolRequestParams::new("devlog_new")
                .with_arguments(rmcp::object!({ "section": 42, "text": "x" })),
        )
        .await;
    // Accept either a protocol-level error or a tool-level error — both
    // satisfy the "don't silently accept junk" contract.  What we care
    // about is that the file did NOT get created.
    if let Ok(r) = &attempt {
        assert!(is_tool_error(r));
    }
    // Nothing named "42" should exist under DEVLOG.
    assert!(!base.path().join("DEVLOG").join("42").exists());
    client.cancel().await.ok();
}

#[tokio::test]
async fn invalid_base_dir_override_read_is_tool_error() {
    // Read against a base_dir that doesn't contain DEVLOG/<section>/ must
    // come back as a tool error, not a protocol blowup.
    let base = fresh_base();
    let nonexistent = base.path().join("no-such-subdir");
    let client = spawn_subprocess_client(base.path()).await;

    let result = call(
        &client,
        "devlog_read",
        json!({
            "section": "core",
            "base_dir": nonexistent.to_string_lossy(),
        }),
    )
    .await;
    assert_wire_err(&result);
    client.cancel().await.ok();
}
