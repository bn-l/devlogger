//! Tests for the `devlog_list` tool.

use super::common::*;

#[tokio::test]
async fn list_single_section_returns_entries_in_order() {
    let (server, _dir) = fresh_server();
    for i in 1..=3 {
        server
            .devlog_new(new_args("parser", &format!("entry {i}")))
            .await
            .unwrap();
    }

    let result = server.devlog_list(list_args(Some("parser"))).await.unwrap();
    assert_ok(&result);

    let s = structured(&result)
        .as_array()
        .expect("expected array")
        .clone();
    assert_eq!(s.len(), 3);
    for (i, v) in s.iter().enumerate() {
        let n = v.get("number").and_then(|x| x.as_u64()).unwrap();
        assert_eq!(n as usize, i + 1);
        let text = v.get("text").and_then(|x| x.as_str()).unwrap();
        assert_eq!(text, format!("entry {}", i + 1));
    }
}

#[tokio::test]
async fn list_missing_section_is_tool_error() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_list(list_args(Some("nonesuch")))
        .await
        .unwrap();
    let msg = assert_err(&result);
    assert!(msg.contains("not found"), "got {msg:?}");
}

#[tokio::test]
async fn list_without_section_and_no_sections_returns_empty_array() {
    let (server, _dir) = fresh_server();
    let result = server.devlog_list(list_args(None)).await.unwrap();
    assert_ok(&result);

    let arr = structured(&result).as_array().unwrap();
    assert!(arr.is_empty(), "expected empty, got {arr:?}");
    assert_eq!(text_of(&result), "");
}

#[tokio::test]
async fn list_without_section_groups_by_section_alphabetically() {
    let (server, _dir) = fresh_server();
    // Insert in non-alphabetical order to verify sort.
    server.devlog_new(new_args("zulu", "z1")).await.unwrap();
    server.devlog_new(new_args("alpha", "a1")).await.unwrap();
    server.devlog_new(new_args("alpha", "a2")).await.unwrap();
    server.devlog_new(new_args("mid", "m1")).await.unwrap();

    let result = server.devlog_list(list_args(None)).await.unwrap();
    assert_ok(&result);

    let groups = structured(&result).as_array().unwrap();
    let names: Vec<&str> = groups
        .iter()
        .map(|g| g.get("section").and_then(|v| v.as_str()).unwrap())
        .collect();
    assert_eq!(names, vec!["alpha", "mid", "zulu"]);

    let alpha_entries = groups[0].get("entries").unwrap().as_array().unwrap();
    assert_eq!(alpha_entries.len(), 2);
}

#[tokio::test]
async fn list_without_section_text_uses_bracket_prefix() {
    let (server, _dir) = fresh_server();
    server.devlog_new(new_args("one", "hello")).await.unwrap();
    server.devlog_new(new_args("two", "world")).await.unwrap();

    let result = server.devlog_list(list_args(None)).await.unwrap();
    assert_ok(&result);

    let text = text_of(&result);
    assert!(text.contains("[one] - 1 | "));
    assert!(text.contains("[two] - 1 | "));
    assert!(text.contains(": hello"));
    assert!(text.contains(": world"));
}

#[tokio::test]
async fn list_with_section_text_matches_entries() {
    let (server, _dir) = fresh_server();
    server.devlog_new(new_args("core", "a")).await.unwrap();
    server.devlog_new(new_args("core", "b")).await.unwrap();
    let result = server.devlog_list(list_args(Some("core"))).await.unwrap();
    assert_ok(&result);

    let text = text_of(&result);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("- 1 | "));
    assert!(lines[1].starts_with("- 2 | "));
}

#[tokio::test]
async fn list_rejects_invalid_section_name() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_list(list_args(Some("BadName")))
        .await
        .unwrap();
    let msg = assert_err(&result);
    assert!(
        msg.contains("invalid section") || msg.contains("illegal"),
        "got {msg}"
    );
}
