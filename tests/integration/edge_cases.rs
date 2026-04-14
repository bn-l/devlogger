//! Miscellaneous edge cases: empty text, whitespace-only text, very long
//! text, no-trailing-newline preservation, read on prose-only file.

use super::common::{main_devlog, run_err, run_ok};

fn seed(path: &std::path::Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn empty_entry_text_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let out = run_ok(dir.path(), &["new", ""]);
    // Entry line ends with "<date>: " plus newline — trailing space after ":"
    // must be preserved, so we check raw (untrimmed) stdout.
    assert!(out.ends_with(": \n"), "got: {out:?}");
    let list = run_ok(dir.path(), &["list"]);
    assert!(list.ends_with(": \n"), "got: {list:?}");
}

#[test]
fn whitespace_only_entry_text_is_accepted_and_preserved() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "   "]);
    let contents = std::fs::read_to_string(main_devlog(dir.path())).unwrap();
    assert!(contents.contains(":    \n") || contents.contains(":    \r\n"),
        "got: {contents:?}");
}

#[test]
fn very_long_entry_text_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let long = "x".repeat(50_000);
    run_ok(dir.path(), &["new", &long]);
    // `list` truncates each row to 80 chars; verify the full text is
    // stored on disk and retrievable verbatim via `read`.
    let read = run_ok(dir.path(), &["read"]);
    assert!(read.contains(&long));
}

#[test]
fn no_trailing_newline_preserved_on_update() {
    let dir = tempfile::tempdir().unwrap();
    let path = main_devlog(dir.path());
    seed(&path, b"- 1 | 2026-04-14 10:00:00: one"); // no trailing \n

    run_ok(dir.path(), &["update", "1", "one-updated"]);

    let bytes = std::fs::read(&path).unwrap();
    assert!(!bytes.ends_with(b"\n"), "no-trailing-newline should be preserved: {bytes:?}");
}

#[test]
fn read_whole_file_dumps_bytes_verbatim_including_prose() {
    let dir = tempfile::tempdir().unwrap();
    let path = main_devlog(dir.path());
    let bytes = b"# Project devlog\n\nSome prose about the project.\n- 1 | 2026-04-14 10:00:00: first\n";
    seed(&path, bytes);

    let out = run_ok(dir.path(), &["read"]);
    assert!(out.contains("# Project devlog"));
    assert!(out.contains("Some prose about the project."));
    assert!(out.contains("- 1 | "));
}

#[test]
fn read_n_skips_prose_and_returns_only_entry_lines() {
    let dir = tempfile::tempdir().unwrap();
    let path = main_devlog(dir.path());
    let bytes = b"# header\n\nprose\n- 1 | 2026-04-14 10:00:00: one\nmore prose\n- 2 | 2026-04-14 11:00:00: two\n";
    seed(&path, bytes);

    let out = run_ok(dir.path(), &["read", "2"]);
    assert!(!out.contains("# header"));
    assert!(!out.contains("prose"));
    assert!(out.contains("- 1 | "));
    assert!(out.contains("- 2 | "));
}

#[test]
fn read_n_on_prose_only_file_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = main_devlog(dir.path());
    seed(&path, b"# header only\n\nno entries in this file yet\n");

    let out = run_ok(dir.path(), &["read", "5"]);
    assert!(out.is_empty(), "got: {out:?}");
}

#[test]
fn update_to_identical_text_is_a_noop_in_content_but_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "same"]);
    let before = std::fs::read(main_devlog(dir.path())).unwrap();

    run_ok(dir.path(), &["update", "1", "same"]);
    let after = std::fs::read(main_devlog(dir.path())).unwrap();

    // The rewrite may re-normalize whitespace, but the observable file
    // content should be unchanged.
    assert_eq!(before, after);
}

#[test]
fn f_flag_accepts_relative_path() {
    let dir = tempfile::tempdir().unwrap();
    // Set CWD to dir's parent; pass a relative path to -f.
    let parent = dir.path().parent().unwrap();
    let rel_name = dir
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let out = std::process::Command::new(super::common::bin())
        .current_dir(parent)
        .args(["-f", &rel_name, "new", "rel-path-entry"])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(main_devlog(dir.path()).exists());
}

#[test]
fn updating_last_entry_works_correctly() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "one"]);
    run_ok(dir.path(), &["new", "two"]);
    run_ok(dir.path(), &["new", "three"]);

    run_ok(dir.path(), &["update", "3", "three-revised"]);

    let list = run_ok(dir.path(), &["list"]);
    assert!(list.contains(": one"));
    assert!(list.contains(": two"));
    assert!(list.contains(": three-revised"));
    assert!(!list.contains(": three\n"));
}

#[test]
fn updating_first_entry_works_correctly() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "one"]);
    run_ok(dir.path(), &["new", "two"]);
    run_ok(dir.path(), &["new", "three"]);

    run_ok(dir.path(), &["update", "1", "one-revised"]);

    let contents = std::fs::read_to_string(main_devlog(dir.path())).unwrap();
    let first_line = contents.lines().next().unwrap();
    assert!(first_line.contains(": one-revised"));
}

#[test]
fn list_on_file_with_only_prose_returns_empty_output() {
    let dir = tempfile::tempdir().unwrap();
    let path = main_devlog(dir.path());
    seed(&path, b"# just a header\n\nno entries yet\n");

    let out = run_ok(dir.path(), &["list"]);
    assert!(out.is_empty(), "got: {out:?}");
}

#[test]
fn update_errors_when_no_entries_exist_even_if_file_present() {
    let dir = tempfile::tempdir().unwrap();
    let path = main_devlog(dir.path());
    seed(&path, b"# just a header\n");

    let stderr = run_err(dir.path(), &["update", "1", "x"]);
    assert!(stderr.contains("no entry with number 1"), "stderr: {stderr}");
}
