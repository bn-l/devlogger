use super::common::{run, run_ok, section_devlog};

#[test]
fn numeric_section_name_error_mentions_the_bad_char() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _, stderr) = run(dir.path(), &["new", "5", "nope"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("invalid section name '5'"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("illegal character '5'"), "stderr: {stderr}");
}

#[test]
fn uppercase_section_name_error() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _, stderr) = run(dir.path(), &["list", "Backend"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("illegal character 'B'"), "stderr: {stderr}");
}

#[test]
fn leading_hyphen_section_name_error() {
    // Bare `-foo` is ambiguous on the command line (clap will try to parse
    // it as a short-flag cluster), so users must escape with `--` to
    // pass a literal `-foo`.  The section-name validator then rejects it.
    let dir = tempfile::tempdir().unwrap();
    let (code, _, stderr) = run(dir.path(), &["list", "--", "-foo"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("must not start with '-'") || stderr.contains("invalid section"),
        "stderr: {stderr}"
    );
}

#[test]
fn trailing_hyphen_section_name_error() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _, stderr) = run(dir.path(), &["list", "foo-"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("must not end with '-'"), "stderr: {stderr}");
}

#[test]
fn consecutive_hyphens_section_name_error() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _, stderr) = run(dir.path(), &["new", "a--b", "x"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("consecutive hyphens"), "stderr: {stderr}");
}

#[test]
fn parse_error_is_single_line_with_path_and_line_number() {
    let dir = tempfile::tempdir().unwrap();
    // Seed a valid section devlog, then corrupt it.
    run_ok(dir.path(), &["new", "main", "ok"]);
    let path = section_devlog(dir.path(), "main");
    let mut contents = std::fs::read_to_string(&path).unwrap();
    contents.push_str("- this is broken\n");
    std::fs::write(&path, &contents).unwrap();

    let (code, _, stderr) = run(dir.path(), &["list", "main"]);
    assert_ne!(code, 0);
    assert!(
        stderr.contains(&path.display().to_string()),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains(":3:") || stderr.contains(":2:"),
        "should include line number: {stderr}"
    );
    assert!(
        stderr.contains("expected `- <number> | "),
        "should describe the format: {stderr}"
    );
}

#[test]
fn parse_error_does_not_dump_file_contents_via_cli() {
    let dir = tempfile::tempdir().unwrap();
    let path = section_devlog(dir.path(), "main");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    // A file with an obvious "secret" we should NOT see echoed back.
    std::fs::write(
        &path,
        "SENTINEL_SECRET_aaa\nline two prose\n- totally broken line\nSENTINEL_SECRET_bbb\n",
    )
    .unwrap();

    let (code, _, stderr) = run(dir.path(), &["list", "main"]);
    assert_ne!(code, 0);
    assert!(
        !stderr.contains("SENTINEL_SECRET_aaa"),
        "file content leaked: {stderr}"
    );
    assert!(
        !stderr.contains("SENTINEL_SECRET_bbb"),
        "file content leaked: {stderr}"
    );
    assert!(
        !stderr.contains("totally broken line"),
        "raw line leaked: {stderr}"
    );
    // Should be short and single-line (plus the `devlogger: ` prefix + newline).
    assert!(
        stderr.lines().count() <= 2,
        "error should be concise: {stderr}"
    );
}

#[test]
fn parse_error_on_read_also_surfaces() {
    let dir = tempfile::tempdir().unwrap();
    let path = section_devlog(dir.path(), "main");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "- 1 | bogus-date: text\n").unwrap();

    let (code, _, stderr) = run(dir.path(), &["read", "main", "1"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("YYYY-MM-DD HH:MM:SS"), "stderr: {stderr}");
}

#[test]
fn read_fails_on_invalid_n_format() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "backend", "x"]);
    let (code, _, stderr) = run(dir.path(), &["read", "backend", "notnum"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("non-negative integer"), "stderr: {stderr}");
}
