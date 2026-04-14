use super::common::{run, run_err, run_ok};

#[test]
fn list_on_missing_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let stderr = run_err(dir.path(), &["list"]);
    assert!(stderr.contains("devlog not found"), "stderr: {stderr}");
}

#[test]
fn list_shows_entries_in_order() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "alpha"]);
    run_ok(dir.path(), &["new", "beta"]);
    run_ok(dir.path(), &["new", "gamma"]);

    let out = run_ok(dir.path(), &["list"]);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains(": alpha"));
    assert!(lines[1].contains(": beta"));
    assert!(lines[2].contains(": gamma"));
}

#[test]
fn list_main_and_section_are_separate() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "main thing"]);
    run_ok(dir.path(), &["new", "backend", "back thing"]);

    let main = run_ok(dir.path(), &["list"]);
    let sect = run_ok(dir.path(), &["list", "backend"]);

    assert!(main.contains("main thing"));
    assert!(!main.contains("back thing"));
    assert!(sect.contains("back thing"));
    assert!(!sect.contains("main thing"));
}

#[test]
fn list_missing_section_errors() {
    let dir = tempfile::tempdir().unwrap();
    let stderr = run_err(dir.path(), &["list", "nonexistent"]);
    assert!(stderr.contains("devlog not found"), "stderr: {stderr}");
}

#[test]
fn list_rejects_invalid_section_name() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _, stderr) = run(dir.path(), &["list", "Foo"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("invalid section name"), "stderr: {stderr}");
}

#[test]
fn list_output_shape_matches_canonical_entry_line() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "hello"]);
    let out = run_ok(dir.path(), &["list"]);
    let first = out.lines().next().unwrap();
    // Shape: "- <N> | <YYYY-MM-DD HH:MM:SS>: <text>"
    assert!(first.starts_with("- 1 | "), "got: {first}");
    assert!(first.contains(": hello"), "got: {first}");
}
