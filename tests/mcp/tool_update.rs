//! Tests for the `devlog_update` tool.

use super::common::*;

/// Seed a section's devlog file directly on disk with two entries whose
/// timestamps are guaranteed distinct.  Going through the CLI would
/// stamp `Local::now()` for both, which collides at second resolution
/// when the test runs fast — making date-based lookups flaky.
async fn seeded_server() -> (
    devlogger::mcp::DevlogServer,
    tempfile::TempDir,
    String,
    String,
) {
    let (server, dir) = fresh_server();
    let path = section_file(dir.path(), "core");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let d1 = "2026-04-14 09:00:00".to_string();
    let d2 = "2026-04-15 18:30:45".to_string();
    let contents = format!("- 1 | {d1}: initial one\n- 2 | {d2}: initial two\n");
    std::fs::write(&path, contents).unwrap();
    (server, dir, d1, d2)
}

#[tokio::test]
async fn update_by_number_replaces_text_and_preserves_id_and_date() {
    let (server, dir, _, _) = seeded_server().await;

    let result = server
        .devlog_update(update_args("core", "1", "revised"))
        .await
        .unwrap();
    assert_ok(&result);

    let s = structured(&result);
    assert_eq!(s.get("number").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(s.get("text").and_then(|v| v.as_str()), Some("revised"));

    let contents = std::fs::read_to_string(section_file(dir.path(), "core")).unwrap();
    assert!(contents.contains(": revised"));
    assert!(!contents.contains(": initial one"));
    // Second entry untouched.
    assert!(contents.contains(": initial two"));
}

#[tokio::test]
async fn update_by_exact_date_targets_the_matching_entry() {
    let (server, _dir, d1, _d2) = seeded_server().await;
    let result = server
        .devlog_update(update_args("core", &d1, "via-exact-date"))
        .await
        .unwrap();
    assert_ok(&result);
    assert_eq!(
        structured(&result).get("number").and_then(|v| v.as_u64()),
        Some(1),
        "exact date should target entry 1"
    );
    assert_eq!(
        structured(&result).get("text").and_then(|v| v.as_str()),
        Some("via-exact-date")
    );
}

#[tokio::test]
async fn update_by_date_prefix_when_unique() {
    let (server, _dir, d1, _d2) = seeded_server().await;
    // Everything before the time component is a valid unique prefix as
    // long as both entries share the same day.  Use the date-only prefix.
    let date_only = &d1[..10];
    let result = server
        .devlog_update(update_args("core", date_only, "via-prefix"))
        .await
        .unwrap();

    if d1[..10] == _d2[..10] {
        // Same day: prefix is ambiguous, expect tool error.
        let msg = assert_err(&result);
        assert!(msg.contains("ambiguous"), "got {msg}");
    } else {
        // Different days: prefix uniquely identifies entry 1.
        assert_ok(&result);
        assert_eq!(
            structured(&result).get("text").and_then(|v| v.as_str()),
            Some("via-prefix")
        );
    }
}

#[tokio::test]
async fn update_unknown_number_is_tool_error() {
    let (server, _dir, _, _) = seeded_server().await;
    let result = server
        .devlog_update(update_args("core", "99", "whatever"))
        .await
        .unwrap();
    let msg = assert_err(&result);
    assert!(msg.contains("no entry"), "got {msg}");
}

#[tokio::test]
async fn update_unknown_id_is_tool_error() {
    let (server, _dir, _, _) = seeded_server().await;
    let result = server
        .devlog_update(update_args("core", "not-a-real-id", "text"))
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn update_missing_section_is_tool_error() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_update(update_args("nope", "1", "replacement"))
        .await
        .unwrap();
    let msg = assert_err(&result);
    assert!(msg.contains("not found"), "got {msg}");
}

#[tokio::test]
async fn update_rejects_multiline_replacement() {
    let (server, _dir, _, _) = seeded_server().await;
    let result = server
        .devlog_update(update_args("core", "1", "bad\nreplacement"))
        .await
        .unwrap();
    let msg = assert_err(&result);
    assert!(
        msg.contains("newline") || msg.contains("control"),
        "got {msg}"
    );
}

#[tokio::test]
async fn update_preserves_number_and_date_exactly() {
    let (server, dir) = fresh_server();
    let r1 = server
        .devlog_new(new_args("core", "original"))
        .await
        .unwrap();
    let original_date = structured(&r1)
        .get("date")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string();

    server
        .devlog_update(update_args("core", "1", "rewritten"))
        .await
        .unwrap();

    let contents = std::fs::read_to_string(section_file(dir.path(), "core")).unwrap();
    let entry_line = contents
        .lines()
        .find(|l| l.starts_with("- 1 | "))
        .expect("entry 1 should still exist");
    assert!(entry_line.contains(&original_date));
    assert!(entry_line.ends_with(": rewritten"));
}
