//! Tests for the `devlog_read` tool.

use super::common::*;

#[tokio::test]
async fn read_entire_file_returns_full_contents() {
    let (server, dir) = fresh_server();
    server.devlog_new(new_args("core", "one")).await.unwrap();
    server.devlog_new(new_args("core", "two")).await.unwrap();

    let disk = std::fs::read_to_string(section_file(dir.path(), "core")).unwrap();

    let result = server.devlog_read(read_args("core", None)).await.unwrap();
    assert_ok(&result);

    let text = text_of(&result);
    assert_eq!(text, disk, "read output must be byte-equal to the file");

    let s = structured(&result);
    assert_eq!(
        s.get("contents").and_then(|v| v.as_str()),
        Some(disk.as_str())
    );
}

#[tokio::test]
async fn read_last_n_returns_only_entry_lines() {
    let (server, dir) = fresh_server();
    for i in 1..=5 {
        server
            .devlog_new(new_args("core", &format!("entry {i}")))
            .await
            .unwrap();
    }

    // Inject a prose line to verify last-N filters to entry-shaped lines only.
    let path = section_file(dir.path(), "core");
    let mut raw = std::fs::read_to_string(&path).unwrap();
    raw.push_str("a random prose line, not an entry\n");
    std::fs::write(&path, raw).unwrap();

    let result = server
        .devlog_read(read_args("core", Some(2)))
        .await
        .unwrap();
    assert_ok(&result);

    let text = text_of(&result);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("- 4 | "));
    assert!(lines[1].starts_with("- 5 | "));
}

#[tokio::test]
async fn read_n_larger_than_total_returns_all_entries() {
    let (server, _dir) = fresh_server();
    server.devlog_new(new_args("core", "only")).await.unwrap();

    let result = server
        .devlog_read(read_args("core", Some(99)))
        .await
        .unwrap();
    assert_ok(&result);

    let text = text_of(&result);
    assert_eq!(text.lines().count(), 1);
    assert!(text.contains(": only"));
}

#[tokio::test]
async fn read_missing_section_is_tool_error() {
    let (server, _dir) = fresh_server();
    let result = server.devlog_read(read_args("ghost", None)).await.unwrap();
    let msg = assert_err(&result);
    assert!(msg.contains("not found"), "got {msg}");
}

#[tokio::test]
async fn read_invalid_section_name_is_tool_error() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_read(read_args("Bad_Name", None))
        .await
        .unwrap();
    let msg = assert_err(&result);
    assert!(
        msg.contains("invalid section") || msg.contains("illegal"),
        "got {msg}"
    );
}
