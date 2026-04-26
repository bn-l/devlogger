//! Tests for the `devlog_new` tool.

use super::common::*;

#[tokio::test]
async fn new_entry_creates_file_and_returns_success_message() {
    let (server, dir) = fresh_server();
    let result = server
        .devlog_new(new_args("parser", "first entry"))
        .await
        .expect("protocol error");
    assert_ok(&result);

    let text = text_of(&result);
    assert!(
        text.contains("#1") && text.contains("successfully created"),
        "expected success message referencing entry #1, got {text:?}"
    );
    // The raw entry line must NOT leak into the response.
    assert!(
        !text.starts_with("- 1 | "),
        "response should not include the canonical entry line, got {text:?}"
    );
    assert!(
        !text.contains(": first entry"),
        "response should not include the entry text, got {text:?}"
    );

    assert!(
        section_file(dir.path(), "parser").is_file(),
        "section file should have been created"
    );
}

#[tokio::test]
async fn new_entry_structured_content_has_number_date_and_message_only() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_new(new_args("core", "hello world"))
        .await
        .unwrap();
    assert_ok(&result);

    let s = structured(&result);
    assert_eq!(s.get("number").and_then(|v| v.as_u64()), Some(1));

    let date = s.get("date").and_then(|v| v.as_str()).unwrap();
    assert_eq!(date.len(), "YYYY-MM-DD HH:MM:SS".len(), "date was {date:?}");

    let message = s.get("message").and_then(|v| v.as_str()).unwrap();
    assert!(message.contains("#1"));
    assert!(message.contains("successfully created"));
    assert!(message.contains(date));

    // Only `number`, `date`, and `message` should be present — no leak of
    // the entry's text or its canonical line.
    let obj = s.as_object().expect("structured result is an object");
    let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(keys, vec!["date", "message", "number"]);
}

#[tokio::test]
async fn new_entry_increments_number_on_successive_calls() {
    let (server, _dir) = fresh_server();
    for expected in 1..=4 {
        let result = server
            .devlog_new(new_args("core", &format!("entry {expected}")))
            .await
            .unwrap();
        assert_ok(&result);
        let n = structured(&result)
            .get("number")
            .and_then(|v| v.as_u64())
            .unwrap();
        assert_eq!(n, expected, "expected monotonically increasing numbering");
    }
}

#[tokio::test]
async fn new_entry_creates_independent_sections() {
    let (server, dir) = fresh_server();
    server
        .devlog_new(new_args("alpha", "a1"))
        .await
        .unwrap();
    server
        .devlog_new(new_args("beta", "b1"))
        .await
        .unwrap();

    assert!(section_file(dir.path(), "alpha").is_file());
    assert!(section_file(dir.path(), "beta").is_file());
}

#[tokio::test]
async fn new_entry_rejects_invalid_section_as_tool_error() {
    let (server, _dir) = fresh_server();
    // Invalid: uppercase letter.
    let result = server
        .devlog_new(new_args("Parser", "text"))
        .await
        .expect("bad section should surface as tool error, not protocol error");
    let msg = assert_err(&result);
    assert!(
        msg.contains("section") || msg.contains("illegal character"),
        "unexpected error message: {msg}"
    );
}

#[tokio::test]
async fn new_entry_rejects_multiline_text_as_tool_error() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_new(new_args("core", "line1\nline2"))
        .await
        .unwrap();
    let msg = assert_err(&result);
    assert!(
        msg.contains("newline") || msg.contains("control"),
        "unexpected error message: {msg}"
    );
}

#[tokio::test]
async fn new_entry_writes_the_entry_to_disk() {
    let (server, dir) = fresh_server();
    server
        .devlog_new(new_args("store", "durable entry"))
        .await
        .unwrap();

    let path = section_file(dir.path(), "store");
    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.contains(": durable entry"), "contents={contents:?}");
    // Entry line must start with the canonical prefix.
    assert!(contents.lines().any(|l| l.starts_with("- 1 | ")));
}
