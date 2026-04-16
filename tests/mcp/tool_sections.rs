//! Tests for the `devlog_sections` tool.

use super::common::*;

#[tokio::test]
async fn sections_empty_base_returns_empty_array() {
    let (server, _dir) = fresh_server();
    let result = server.devlog_sections(sections_args()).await.unwrap();
    assert_ok(&result);

    let arr = structured(&result).as_array().unwrap();
    assert!(arr.is_empty());
    assert_eq!(text_of(&result), "");
}

#[tokio::test]
async fn sections_returns_sorted_unique_names() {
    let (server, _dir) = fresh_server();
    server.devlog_new(new_args("zulu", "z1")).await.unwrap();
    server.devlog_new(new_args("alpha", "a1")).await.unwrap();
    server.devlog_new(new_args("mid", "m1")).await.unwrap();
    server.devlog_new(new_args("alpha", "a2")).await.unwrap();

    let result = server.devlog_sections(sections_args()).await.unwrap();
    assert_ok(&result);

    let arr = structured(&result).as_array().unwrap();
    let names: Vec<&str> = arr.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(names, vec!["alpha", "mid", "zulu"]);
}

#[tokio::test]
async fn sections_text_is_one_name_per_line() {
    let (server, _dir) = fresh_server();
    server.devlog_new(new_args("one", "x")).await.unwrap();
    server.devlog_new(new_args("two", "y")).await.unwrap();

    let result = server.devlog_sections(sections_args()).await.unwrap();
    assert_eq!(text_of(&result), "one\ntwo");
}

#[tokio::test]
async fn sections_ignores_invalidly_named_directories() {
    let (server, dir) = fresh_server();
    server.devlog_new(new_args("valid", "x")).await.unwrap();

    // Drop a directory with an invalid name into DEVLOG/; sections must
    // quietly skip it.
    std::fs::create_dir_all(dir.path().join("DEVLOG").join("NotValid")).unwrap();

    let result = server.devlog_sections(sections_args()).await.unwrap();
    assert_ok(&result);
    let arr = structured(&result).as_array().unwrap();
    let names: Vec<&str> = arr.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(names, vec!["valid"]);
}

#[tokio::test]
async fn sections_ignores_section_dirs_without_a_devlog_file() {
    let (server, dir) = fresh_server();
    server.devlog_new(new_args("real", "x")).await.unwrap();

    // Orphan section directory — valid name, but no `<section>-devlog.md`.
    std::fs::create_dir_all(dir.path().join("DEVLOG").join("orphan")).unwrap();

    let result = server.devlog_sections(sections_args()).await.unwrap();
    let arr = structured(&result).as_array().unwrap();
    let names: Vec<&str> = arr.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(names, vec!["real"]);
}
