//! A file with duplicate entry numbers can arise from hand-editing (or
//! from historic races on an old version of the tool).  `update` must not
//! silently rewrite the wrong line: it resolves by entry position, and
//! rejects a bare-number id that matches more than one entry.

use super::common::{run, run_err, run_ok, section_devlog};

fn seed(path: &std::path::Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

#[test]
fn update_by_exact_date_targets_correct_entry_even_with_duplicate_numbers() {
    let dir = tempfile::tempdir().unwrap();
    let path = section_devlog(dir.path(), "main");
    seed(
        &path,
        "- 1 | 2026-04-14 10:00:00: first original\n\
         - 1 | 2026-04-14 11:00:00: second original\n",
    );

    run_ok(
        dir.path(),
        &["update", "main", "2026-04-14 11:00:00", "SECOND UPDATED"],
    );

    let after = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = after.lines().collect();
    // First line untouched: original date + original text.
    assert_eq!(
        lines[0], "- 1 | 2026-04-14 10:00:00: first original",
        "first line should be untouched"
    );
    // Second line has its text updated but number + date preserved.
    assert_eq!(
        lines[1], "- 1 | 2026-04-14 11:00:00: SECOND UPDATED",
        "second line should be the one rewritten"
    );
}

#[test]
fn update_by_ambiguous_number_errors_with_guidance() {
    let dir = tempfile::tempdir().unwrap();
    seed(
        &section_devlog(dir.path(), "main"),
        "- 1 | 2026-04-14 10:00:00: a\n- 1 | 2026-04-14 11:00:00: b\n",
    );

    let stderr = run_err(dir.path(), &["update", "main", "1", "nope"]);
    assert!(stderr.contains("ambiguous"), "stderr: {stderr}");
    assert!(stderr.contains("number 1"), "stderr: {stderr}");
    assert!(stderr.contains("exact date"), "stderr: {stderr}");
}

#[test]
fn update_by_ambiguous_number_does_not_mutate_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = section_devlog(dir.path(), "main");
    let before = "- 1 | 2026-04-14 10:00:00: a\n- 1 | 2026-04-14 11:00:00: b\n";
    seed(&path, before);

    let (code, _, _) = run(dir.path(), &["update", "main", "1", "nope"]);
    assert_ne!(code, 0);

    let after = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        before, after,
        "file should be untouched after ambiguity error"
    );
}

#[test]
fn update_by_date_prefix_errors_when_multiple_entries_match() {
    let dir = tempfile::tempdir().unwrap();
    seed(
        &section_devlog(dir.path(), "main"),
        "- 1 | 2026-04-14 10:00:00: a\n- 2 | 2026-04-14 11:00:00: b\n",
    );

    let stderr = run_err(dir.path(), &["update", "main", "2026-04-14", "nope"]);
    assert!(stderr.contains("ambiguous date prefix"), "stderr: {stderr}");
}

#[test]
fn list_still_works_on_file_with_duplicate_numbers() {
    // Reading/listing should tolerate duplicate numbers — only updating
    // them by number is rejected.
    let dir = tempfile::tempdir().unwrap();
    seed(
        &section_devlog(dir.path(), "main"),
        "- 1 | 2026-04-14 10:00:00: a\n- 1 | 2026-04-14 11:00:00: b\n",
    );

    let out = run_ok(dir.path(), &["list", "main"]);
    assert!(out.contains(": a"));
    assert!(out.contains(": b"));
}
