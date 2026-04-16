//! Tests for the `base_dir` per-call override.  The server has a default
//! base (configured with `--dir` at boot).  Every tool can override that
//! on a single call — which must land the write in the override directory,
//! leaving the default untouched, and vice-versa.

use super::common::*;

#[tokio::test]
async fn new_writes_to_default_base_when_override_omitted() {
    let (server, dir) = fresh_server();
    server.devlog_new(new_args("core", "x")).await.unwrap();
    assert!(section_file(dir.path(), "core").is_file());
}

#[tokio::test]
async fn new_honours_base_dir_override() {
    let default_dir = tempfile::tempdir().unwrap();
    let override_dir = tempfile::tempdir().unwrap();

    let server = server_at(default_dir.path());
    server
        .devlog_new(new_args_at("alt", "entry", override_dir.path()))
        .await
        .unwrap();

    assert!(
        section_file(override_dir.path(), "alt").is_file(),
        "write must land in the override directory"
    );
    assert!(
        !section_file(default_dir.path(), "alt").exists(),
        "default must be untouched"
    );
}

#[tokio::test]
async fn list_reads_from_override_base_dir() {
    let default_dir = tempfile::tempdir().unwrap();
    let override_dir = tempfile::tempdir().unwrap();
    let server = server_at(default_dir.path());

    // Populate only the override base.
    server
        .devlog_new(new_args_at("core", "a", override_dir.path()))
        .await
        .unwrap();

    // Override list call sees the entry…
    let result = server
        .devlog_list(list_args_at(Some("core"), override_dir.path()))
        .await
        .unwrap();
    assert_ok(&result);
    let arr = structured(&result).as_array().unwrap();
    assert_eq!(arr.len(), 1);

    // …but the default-scoped list call does not.
    let result2 = server.devlog_list(list_args(Some("core"))).await.unwrap();
    assert!(is_error(&result2), "default base should not see the section");
}

#[tokio::test]
async fn sections_honours_override_base_dir() {
    let default_dir = tempfile::tempdir().unwrap();
    let override_dir = tempfile::tempdir().unwrap();
    let server = server_at(default_dir.path());

    server
        .devlog_new(new_args_at("only-here", "x", override_dir.path()))
        .await
        .unwrap();

    let default_sections = server.devlog_sections(sections_args()).await.unwrap();
    assert_eq!(
        structured(&default_sections).as_array().unwrap().len(),
        0,
        "default base should be empty"
    );

    let overridden = server
        .devlog_sections(sections_args_at(override_dir.path()))
        .await
        .unwrap();
    let names: Vec<&str> = structured(&overridden)
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["only-here"]);
}

#[tokio::test]
async fn empty_string_base_dir_falls_back_to_default() {
    use rmcp::handler::server::wrapper::Parameters;
    use devlogger::mcp::args::NewArgs;

    let (server, dir) = fresh_server();
    let params = Parameters(NewArgs {
        section: "core".into(),
        text: "via-empty".into(),
        base_dir: Some(String::new()),
    });
    let result = server.devlog_new(params).await.unwrap();
    assert_ok(&result);
    assert!(
        section_file(dir.path(), "core").is_file(),
        "empty base_dir should fall back to the server default"
    );
}

#[tokio::test]
async fn default_base_accessor_returns_configured_path() {
    let tmp = tempfile::tempdir().unwrap();
    let server = server_at(tmp.path());
    assert_eq!(server.default_base(), tmp.path());
}
