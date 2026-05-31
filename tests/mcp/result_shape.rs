//! Tests pinning down the exact shape of every tool's `CallToolResult`.
//!
//! MCP clients in the wild tend to consume either the `content` array
//! (older/simpler clients) or the `structured_content` JSON (newer
//! typed clients).  These tests lock in that every tool populates
//! **both** so both kinds of client get a useful answer.

use super::common::*;

#[tokio::test]
async fn new_result_has_both_text_and_structured() {
    let (server, _dir) = fresh_server();
    let result = server.devlog_new(new_args("core", "x")).await.unwrap();
    assert!(!result.content.is_empty(), "should have content");
    assert!(
        result.structured_content.is_some(),
        "should have structured_content"
    );
    // is_error must be None or Some(false) for a successful call.
    assert!(!is_error(&result));
}

#[tokio::test]
async fn list_single_section_structured_is_object_with_entries() {
    let (server, _dir) = fresh_server();
    server.devlog_new(new_args("core", "x")).await.unwrap();

    let result = server.devlog_list(list_args(Some("core"))).await.unwrap();
    let s = structured(&result);
    assert!(s.is_object(), "single-section list should return a JSON object");
    let entries = s
        .get("entries")
        .expect("missing `entries` key")
        .as_array()
        .expect("entries should be an array");

    let first = &entries[0];
    for field in ["number", "date", "text", "line"] {
        assert!(first.get(field).is_some(), "missing `{field}` in entry");
    }
}

#[tokio::test]
async fn list_all_sections_structured_is_grouped() {
    let (server, _dir) = fresh_server();
    server.devlog_new(new_args("a", "one")).await.unwrap();
    server.devlog_new(new_args("b", "two")).await.unwrap();

    let result = server.devlog_list(list_args(None)).await.unwrap();
    let s = structured(&result);
    assert!(s.is_object(), "all-sections list should return a JSON object");
    let arr = s
        .get("sections")
        .expect("missing `sections` key")
        .as_array()
        .expect("sections should be an array");
    assert_eq!(arr.len(), 2);

    for group in arr {
        assert!(group.get("section").is_some());
        assert!(group.get("entries").is_some());
        let entries = group.get("entries").unwrap().as_array().unwrap();
        assert_eq!(entries.len(), 1);
    }
}

#[tokio::test]
async fn sections_structured_is_array_of_strings() {
    let (server, _dir) = fresh_server();
    server.devlog_new(new_args("alpha", "x")).await.unwrap();
    server.devlog_new(new_args("beta", "y")).await.unwrap();

    let result = server.devlog_sections(sections_args()).await.unwrap();
    let s = structured(&result);
    assert!(s.is_object(), "sections should return a JSON object");
    let arr = s
        .get("sections")
        .expect("missing `sections` key")
        .as_array()
        .expect("sections should be an array");
    for v in arr {
        assert!(v.is_string(), "every element should be a string");
    }
}

#[tokio::test]
async fn read_structured_wraps_contents_in_object() {
    let (server, _dir) = fresh_server();
    server.devlog_new(new_args("core", "x")).await.unwrap();

    let result = server.devlog_read(read_args("core", None)).await.unwrap();
    let s = structured(&result);
    assert!(s.is_object());
    assert!(s.get("contents").and_then(|v| v.as_str()).is_some());
}

#[tokio::test]
async fn tool_errors_flip_is_error_and_still_carry_content() {
    let (server, _dir) = fresh_server();
    let result = server.devlog_read(read_args("nope", None)).await.unwrap();
    assert!(is_error(&result));
    assert!(!result.content.is_empty(), "error must carry a message");
    let msg = text_of(&result);
    assert!(!msg.is_empty());
}

#[tokio::test]
async fn structured_content_is_never_a_bare_array() {
    let (server, _dir) = fresh_server();
    server.devlog_new(new_args("alpha", "x")).await.unwrap();
    server.devlog_new(new_args("beta", "y")).await.unwrap();

    let cases: Vec<(&str, serde_json::Value)> = vec![
        (
            "devlog_new",
            structured(&server.devlog_new(new_args("gamma", "z")).await.unwrap()).clone(),
        ),
        (
            "devlog_list (single section)",
            structured(&server.devlog_list(list_args(Some("alpha"))).await.unwrap()).clone(),
        ),
        (
            "devlog_list (all sections)",
            structured(&server.devlog_list(list_args(None)).await.unwrap()).clone(),
        ),
        (
            "devlog_sections",
            structured(&server.devlog_sections(sections_args()).await.unwrap()).clone(),
        ),
        (
            "devlog_read",
            structured(&server.devlog_read(read_args("alpha", None)).await.unwrap()).clone(),
        ),
        (
            "devlog_update",
            structured(
                &server
                    .devlog_update(update_args("alpha", "1", "edited"))
                    .await
                    .unwrap(),
            )
            .clone(),
        ),
    ];

    for (label, value) in &cases {
        assert!(
            value.is_object(),
            "{label}: structured_content must be a JSON object, got {value}",
        );
    }
}

#[tokio::test]
async fn update_result_carries_full_entry_fields() {
    let (server, _dir) = fresh_server();
    server.devlog_new(new_args("core", "orig")).await.unwrap();
    let result = server
        .devlog_update(update_args("core", "1", "updated"))
        .await
        .unwrap();
    assert!(!result.content.is_empty());
    let s = structured(&result);
    for field in ["number", "date", "text", "line"] {
        assert!(s.get(field).is_some(), "missing `{field}` in update result");
    }
}
