use std::path::{Path, PathBuf};

use devlogger::section::section_devlog_path;
use devlogger::store::load_entries;

#[test]
fn section_path_nests_section_name_twice() {
    let base = Path::new("/tmp/proj");
    let p = section_devlog_path(base, "backend");
    assert_eq!(
        p,
        PathBuf::from("/tmp/proj/DEVLOG/backend/backend-devlog.md")
    );
}

#[test]
fn section_path_uses_section_name_verbatim() {
    let p = section_devlog_path(Path::new("/r"), "api-v-two");
    assert_eq!(p, PathBuf::from("/r/DEVLOG/api-v-two/api-v-two-devlog.md"));
}

#[test]
fn load_entries_returns_empty_for_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("does-not-exist.md");
    let entries = load_entries(&missing).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn load_entries_reads_well_formed_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.md");
    std::fs::write(
        &path,
        "- 1 | 2026-04-14 11:00:00: one\n- 2 | 2026-04-14 11:01:00: two\n",
    )
    .unwrap();
    let entries = load_entries(&path).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[1].text, "two");
}
