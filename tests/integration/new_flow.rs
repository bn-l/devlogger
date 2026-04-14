use super::common::{bin, count_entries, main_devlog, run, run_ok, section_devlog};

#[test]
fn first_new_creates_devlog_directory_and_file() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path();
    assert!(!base.join("DEVLOG").exists());

    run_ok(base, &["new", "first entry"]);

    assert!(base.join("DEVLOG").exists(), "DEVLOG dir should be created");
    assert!(main_devlog(base).exists(), "main-devlog.md should be created");
}

#[test]
fn first_entry_numbered_one() {
    let dir = tempfile::tempdir().unwrap();
    let stdout = run_ok(dir.path(), &["new", "hello"]);
    assert!(stdout.starts_with("- 1 | "), "first entry should be #1: {stdout}");
    assert!(stdout.trim_end().ends_with(": hello"));
}

#[test]
fn sequential_numbering() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "one"]);
    run_ok(dir.path(), &["new", "two"]);
    run_ok(dir.path(), &["new", "three"]);

    let contents = std::fs::read_to_string(main_devlog(dir.path())).unwrap();
    assert_eq!(count_entries(&main_devlog(dir.path())), 3);
    assert!(contents.contains("- 1 | "));
    assert!(contents.contains("- 2 | "));
    assert!(contents.contains("- 3 | "));
}

#[test]
fn each_entry_on_its_own_line() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "one"]);
    run_ok(dir.path(), &["new", "two"]);

    let contents = std::fs::read_to_string(main_devlog(dir.path())).unwrap();
    let entry_lines: Vec<&str> = contents.lines().filter(|l| l.starts_with("- ")).collect();
    assert_eq!(entry_lines.len(), 2);
}

#[test]
fn entry_text_with_colons_survives_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "deploy: staging"]);
    let listing = run_ok(dir.path(), &["list"]);
    assert!(listing.contains(": deploy: staging"), "got: {listing}");
}

#[test]
fn entry_text_with_pipes_survives_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "left | right"]);
    let listing = run_ok(dir.path(), &["list"]);
    assert!(listing.contains("left | right"), "got: {listing}");
}

#[test]
fn section_new_creates_section_subdir() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "backend", "api work"]);

    let sp = section_devlog(dir.path(), "backend");
    assert!(sp.exists(), "section file should exist at {}", sp.display());
    assert!(sp.parent().unwrap().is_dir());
}

#[test]
fn section_numbering_is_independent_from_main() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "main one"]);
    run_ok(dir.path(), &["new", "main two"]);
    run_ok(dir.path(), &["new", "backend", "back one"]);

    // Main has 2 entries starting at 1; backend has 1 entry starting at 1.
    let main_contents = std::fs::read_to_string(main_devlog(dir.path())).unwrap();
    let back_contents =
        std::fs::read_to_string(section_devlog(dir.path(), "backend")).unwrap();
    assert!(main_contents.contains("- 1 | "));
    assert!(main_contents.contains("- 2 | "));
    assert!(back_contents.contains("- 1 | "));
    assert!(!back_contents.contains("- 2 | "));
}

#[test]
fn new_without_dash_f_uses_cwd() {
    // When -f is not supplied, devlogger uses CWD. We set the child's CWD
    // to the temp dir via Command's current_dir.
    let dir = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(bin())
        .current_dir(dir.path())
        .args(["new", "cwd entry"])
        .output()
        .unwrap();
    assert!(output.status.success(), "stderr: {:?}", String::from_utf8_lossy(&output.stderr));
    assert!(main_devlog(dir.path()).exists());
}

#[test]
fn new_appends_rather_than_overwriting() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "one"]);
    let after_one = std::fs::read_to_string(main_devlog(dir.path())).unwrap();
    run_ok(dir.path(), &["new", "two"]);
    let after_two = std::fs::read_to_string(main_devlog(dir.path())).unwrap();
    assert!(
        after_two.starts_with(after_one.trim_end()),
        "after=`{after_two}`, before=`{after_one}`"
    );
}

#[test]
fn new_fails_on_invalid_section_name() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _out, stderr) = run(dir.path(), &["new", "5", "nope"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("invalid section name"), "stderr: {stderr}");
}
