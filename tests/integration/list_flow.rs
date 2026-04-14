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

#[test]
fn list_truncates_long_entries_to_80_chars_with_suffix() {
    let dir = tempfile::tempdir().unwrap();
    let long = "x".repeat(200);
    run_ok(dir.path(), &["new", &long]);

    let out = run_ok(dir.path(), &["list"]);
    let line = out.lines().next().unwrap();

    assert!(
        line.chars().count() <= 80,
        "line is {} chars: {line}",
        line.chars().count()
    );
    assert!(line.ends_with(" more)"), "got: {line}");
    assert!(line.contains("(..."), "got: {line}");
}

#[test]
fn list_does_not_truncate_short_entries() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "short"]);

    let out = run_ok(dir.path(), &["list"]);
    let line = out.lines().next().unwrap();

    assert!(!line.contains("more)"), "got: {line}");
    assert!(line.ends_with(": short"), "got: {line}");
}

#[test]
fn list_truncates_by_display_width_for_wide_glyphs() {
    // 100 CJK chars = 200 display columns raw; list output must fit in 80.
    let dir = tempfile::tempdir().unwrap();
    let wide: String = "漢".repeat(100);
    run_ok(dir.path(), &["new", &wide]);

    let out = run_ok(dir.path(), &["list"]);
    let line = out.lines().next().unwrap();

    // Compute display width the same way the binary does.
    use unicode_width::UnicodeWidthStr;
    assert!(
        line.width() <= 80,
        "line width is {} cols, expected ≤ 80: {line}",
        line.width()
    );
    assert!(line.ends_with(" more)"), "got: {line}");
}

#[test]
fn list_elided_count_is_accurate() {
    let dir = tempfile::tempdir().unwrap();
    // 200-char text; full rendered line is `- 1 | YYYY-MM-DD HH:MM:SS: `
    // (27 chars of prefix) + 200 = 227 chars total.
    let long = "x".repeat(200);
    run_ok(dir.path(), &["new", &long]);

    let out = run_ok(dir.path(), &["list"]);
    let line = out.lines().next().unwrap();

    let open = line.rfind("(...").unwrap();
    let close = line.rfind(" more)").unwrap();
    let reported: usize = line[open + 4..close].parse().unwrap();

    // Reconstruct: chars in line minus the " (...N more)" suffix = kept.
    let suffix_chars = line[open..].chars().count() + 1;
    let kept = line.chars().count() - suffix_chars;
    assert_eq!(kept + reported, 227);
}
