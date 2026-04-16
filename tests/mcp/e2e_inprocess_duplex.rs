//! Full MCP round-trip using rmcp's idiomatic in-process transport —
//! `tokio::io::duplex` — instead of a real subprocess.  Same rmcp
//! protocol layer (initialize handshake, JSON serialization, tools/list,
//! tools/call); just skips the fork and pipe-plumbing cost.  Mirrors
//! rust-sdk's own tests under `crates/rmcp/tests/`.
//!
//! Having the same assertions run over two independent transports
//! isolates "the server is broken" from "the subprocess plumbing is
//! broken" when either suite regresses.

use rmcp::model::CallToolRequestParams;
use rmcp::object;

use super::e2e_common::*;

#[tokio::test]
async fn inprocess_handshake_and_tools_list_match_subprocess() {
    let base = fresh_base();
    let client = spawn_inprocess_client(base.path()).await;

    let tools = client.list_all_tools().await.unwrap();
    let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "devlog_list",
            "devlog_new",
            "devlog_read",
            "devlog_sections",
            "devlog_update",
        ]
    );

    client.cancel().await.ok();
}

#[tokio::test]
async fn inprocess_full_tool_round_trip() {
    // Exercise every tool back-to-back over the duplex transport.  If
    // the duplex path regresses (rmcp version bump, serialization
    // change) this fails before the slower subprocess suite notices.
    let base = fresh_base();
    let client = spawn_inprocess_client(base.path()).await;

    // new
    let new = call_new(&client, "dup", "first").await;
    assert_wire_ok(&new);
    assert_eq!(
        structured(&new).get("number").and_then(|v| v.as_u64()),
        Some(1)
    );

    // sections
    let sections = call_sections(&client).await;
    assert_wire_ok(&sections);
    let names: Vec<&str> = structured(&sections)
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["dup"]);

    // list with section
    let listed = call_list(&client, Some("dup")).await;
    assert_wire_ok(&listed);
    assert_eq!(structured(&listed).as_array().unwrap().len(), 1);

    // update
    let updated = call_update(&client, "dup", "1", "second").await;
    assert_wire_ok(&updated);
    assert_eq!(
        structured(&updated).get("text").and_then(|v| v.as_str()),
        Some("second")
    );

    // read
    let read = call_read(&client, "dup", None).await;
    assert_wire_ok(&read);
    assert!(text_content(&read).contains(": second"));

    client.cancel().await.ok();
}

#[tokio::test]
async fn inprocess_two_clients_with_separate_base_dirs_do_not_cross_contaminate() {
    // Two in-process servers, two duplex pipes, two tempdirs — confirms
    // the in-memory pattern isn't accidentally sharing state through
    // some process-global.
    let base_a = fresh_base();
    let base_b = fresh_base();

    let client_a = spawn_inprocess_client(base_a.path()).await;
    let client_b = spawn_inprocess_client(base_b.path()).await;

    assert_wire_ok(&call_new(&client_a, "left", "in-a").await);
    assert_wire_ok(&call_new(&client_b, "right", "in-b").await);

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
async fn inprocess_tool_error_surfaces_as_wire_error() {
    // Protocol returns OK, but the CallToolResult carries is_error=true
    // and a readable message.  Mirrors the subprocess suite's error
    // expectations so a regression in the error path shows up in both.
    let base = fresh_base();
    let client = spawn_inprocess_client(base.path()).await;

    let result = client
        .call_tool(
            CallToolRequestParams::new("devlog_list")
                .with_arguments(object!({ "section": "nope" })),
        )
        .await
        .expect("tools/call protocol error");

    assert!(matches!(result.is_error, Some(true)));
    let msg = text_content(&result);
    assert!(
        msg.to_lowercase().contains("nope") || msg.to_lowercase().contains("not found"),
        "expected missing-section error, got: {msg}"
    );

    client.cancel().await.ok();
}
