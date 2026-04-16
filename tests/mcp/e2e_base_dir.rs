//! The `base_dir` override argument must work across every tool when
//! invoked over the wire.  Direct-call unit tests already cover the
//! in-process case; this suite confirms serialization+deserialization of
//! the optional string field survives the JSON-RPC round trip.

use serde_json::json;

use super::e2e_common::*;

#[tokio::test]
async fn new_override_routes_write_to_override_directory() {
    let default_base = fresh_base();
    let override_base = fresh_base();
    let client = spawn_subprocess_client(default_base.path()).await;

    let result = call(
        &client,
        "devlog_new",
        json!({
            "section": "alt",
            "text": "overridden",
            "base_dir": override_base.path().to_string_lossy(),
        }),
    )
    .await;
    assert_wire_ok(&result);

    assert!(section_file(override_base.path(), "alt").is_file());
    assert!(!section_file(default_base.path(), "alt").exists());
    client.cancel().await.ok();
}

#[tokio::test]
async fn list_override_reads_from_override_directory() {
    let default_base = fresh_base();
    let override_base = fresh_base();
    let client = spawn_subprocess_client(default_base.path()).await;

    // Seed via an override new.
    call(
        &client,
        "devlog_new",
        json!({
            "section": "core",
            "text": "in-override",
            "base_dir": override_base.path().to_string_lossy(),
        }),
    )
    .await;

    let result = call(
        &client,
        "devlog_list",
        json!({
            "section": "core",
            "base_dir": override_base.path().to_string_lossy(),
        }),
    )
    .await;
    assert_wire_ok(&result);
    let arr = structured(&result).as_array().unwrap();
    assert_eq!(arr.len(), 1);

    // Same call without the override must fail to find the section in
    // the default base.
    let fallback = call_list(&client, Some("core")).await;
    assert_wire_err(&fallback);
    client.cancel().await.ok();
}

#[tokio::test]
async fn sections_override_reflects_override_layout() {
    let default_base = fresh_base();
    let override_base = fresh_base();
    let client = spawn_subprocess_client(default_base.path()).await;

    call(
        &client,
        "devlog_new",
        json!({
            "section": "only-here",
            "text": "x",
            "base_dir": override_base.path().to_string_lossy(),
        }),
    )
    .await;

    // Default sections list is empty.
    let defaults = call_sections(&client).await;
    assert!(structured(&defaults).as_array().unwrap().is_empty());

    let overridden = call(
        &client,
        "devlog_sections",
        json!({ "base_dir": override_base.path().to_string_lossy() }),
    )
    .await;
    assert_wire_ok(&overridden);
    let names: Vec<&str> = structured(&overridden)
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["only-here"]);
    client.cancel().await.ok();
}

#[tokio::test]
async fn read_and_update_honour_override_base_dir() {
    let default_base = fresh_base();
    let override_base = fresh_base();
    let client = spawn_subprocess_client(default_base.path()).await;

    call(
        &client,
        "devlog_new",
        json!({
            "section": "ops",
            "text": "before",
            "base_dir": override_base.path().to_string_lossy(),
        }),
    )
    .await;

    let read = call(
        &client,
        "devlog_read",
        json!({
            "section": "ops",
            "base_dir": override_base.path().to_string_lossy(),
        }),
    )
    .await;
    assert_wire_ok(&read);
    assert!(text_content(&read).contains(": before"));

    let updated = call(
        &client,
        "devlog_update",
        json!({
            "section": "ops",
            "id": "1",
            "text": "after",
            "base_dir": override_base.path().to_string_lossy(),
        }),
    )
    .await;
    assert_wire_ok(&updated);
    assert_eq!(
        structured(&updated).get("text").and_then(|v| v.as_str()),
        Some("after")
    );
    client.cancel().await.ok();
}

#[tokio::test]
async fn empty_string_base_dir_falls_back_to_default_over_wire() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    let result = call(
        &client,
        "devlog_new",
        json!({ "section": "core", "text": "x", "base_dir": "" }),
    )
    .await;
    assert_wire_ok(&result);
    assert!(
        section_file(base.path(), "core").is_file(),
        "empty base_dir should fall back to --dir"
    );
    client.cancel().await.ok();
}
