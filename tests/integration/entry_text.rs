//! Entry text arriving via argv must not contain characters that would
//! break the single-line-per-entry file format.

use super::common::{run, run_ok, section_devlog};

#[test]
fn new_rejects_entry_with_newline() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _, stderr) = run(dir.path(), &["new", "main", "line1\nline2"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("newline"), "stderr: {stderr}");
    // File must not have been touched.
    assert!(!section_devlog(dir.path(), "main").exists());
}

#[test]
fn new_rejects_entry_with_carriage_return() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _, stderr) = run(dir.path(), &["new", "main", "line1\rline2"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("carriage return"), "stderr: {stderr}");
}

#[test]
fn new_rejects_entry_with_crlf() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _, _) = run(dir.path(), &["new", "main", "a\r\nb"]);
    assert_ne!(code, 0);
}

// Note: a literal null byte cannot be passed via argv at the OS level
// (both POSIX `exec*` and `CreateProcess` reject it), so there is no way
// to hand devlogger a null-containing entry from the command line.  The
// library-level protection is exercised in the unit-test suite
// (`unit_tests/entry_text_validation.rs::rejects_null_byte`).

#[test]
fn new_rejects_arbitrary_control_char() {
    let dir = tempfile::tempdir().unwrap();
    // ESC / 0x1B
    let (code, _, stderr) = run(dir.path(), &["new", "main", "a\x1bb"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("U+001B"), "stderr: {stderr}");
}

#[test]
fn new_accepts_tab_in_entry() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "main", "col1\tcol2"]);
    let list = run_ok(dir.path(), &["list", "main"]);
    assert!(list.contains("col1\tcol2"), "got: {list:?}");
}

#[test]
fn new_accepts_unicode_entry() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "main", "café 漢字 🚀"]);
    let list = run_ok(dir.path(), &["list", "main"]);
    assert!(list.contains("café 漢字 🚀"));
}

#[test]
fn update_rejects_entry_with_newline() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "main", "clean"]);
    let (code, _, stderr) = run(dir.path(), &["update", "main", "1", "new\nline"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("newline"), "stderr: {stderr}");

    // Original file unchanged.
    let contents = std::fs::read_to_string(section_devlog(dir.path(), "main")).unwrap();
    assert!(contents.contains(": clean"));
}

#[test]
fn update_rejects_entry_with_carriage_return() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "main", "clean"]);
    let (code, _, stderr) = run(dir.path(), &["update", "main", "1", "a\rb"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("carriage return"), "stderr: {stderr}");
}
