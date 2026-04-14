use super::common::{main_devlog, run_ok, section_devlog};

#[test]
fn main_devlog_lives_at_devlog_main_devlog_md() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "x"]);
    assert_eq!(main_devlog(dir.path()), dir.path().join("DEVLOG/main-devlog.md"));
    assert!(main_devlog(dir.path()).is_file());
}

#[test]
fn section_devlog_nests_section_subdirectory() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "ops", "x"]);
    let sp = section_devlog(dir.path(), "ops");
    assert_eq!(sp, dir.path().join("DEVLOG/ops/ops-devlog.md"));
    assert!(sp.is_file());
    assert!(sp.parent().unwrap().is_dir());
}

#[test]
fn devlog_folder_is_named_uppercase_devlog() {
    // On case-insensitive filesystems (macOS APFS) `join("devlog")` would
    // resolve to the same dir as `DEVLOG`, so check the actual on-disk
    // name by reading the parent directory.
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "x"]);
    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.contains(&"DEVLOG".to_string()), "got: {names:?}");
    assert!(!names.contains(&"devlog".to_string()), "got: {names:?}");
}

#[test]
fn section_file_matches_section_name_convention() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "my-section", "x"]);
    assert!(
        dir.path()
            .join("DEVLOG/my-section/my-section-devlog.md")
            .is_file()
    );
}

#[test]
fn creates_parents_when_devlog_dir_missing() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!dir.path().join("DEVLOG").exists());
    run_ok(dir.path(), &["new", "backend", "x"]);
    assert!(dir.path().join("DEVLOG/backend").is_dir());
}

#[test]
fn file_ends_with_newline() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "x"]);
    let contents = std::fs::read_to_string(main_devlog(dir.path())).unwrap();
    assert!(contents.ends_with('\n'), "file should end with newline: {contents:?}");
}

#[test]
fn no_temp_file_left_behind_after_update() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "one"]);
    run_ok(dir.path(), &["update", "1", "two"]);

    let devlog_dir = dir.path().join("DEVLOG");
    let leftovers: Vec<_> = std::fs::read_dir(&devlog_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let n = e.file_name();
            let s = n.to_string_lossy();
            s.contains(".devlogger.tmp")
        })
        .collect();
    assert!(leftovers.is_empty(), "leftover tmp files: {leftovers:?}");
}
