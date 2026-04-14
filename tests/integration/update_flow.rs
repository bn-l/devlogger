use super::common::{main_devlog, run, run_err, run_ok};

fn first_entry_date(stdout: &str) -> String {
    // "- 1 | 2026-04-14 11:02:37: text" -> "2026-04-14 11:02:37"
    let line = stdout.lines().next().unwrap();
    let after_pipe = line.split_once(" | ").unwrap().1;
    after_pipe.split_once(": ").unwrap().0.to_string()
}

#[test]
fn update_by_number_replaces_text() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "old text"]);
    let out = run_ok(dir.path(), &["update", "1", "new text"]);
    assert!(out.contains(": new text"), "got: {out}");

    let contents = std::fs::read_to_string(main_devlog(dir.path())).unwrap();
    assert!(contents.contains(": new text"));
    assert!(!contents.contains(": old text"));
}

#[test]
fn update_preserves_other_entries() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "one"]);
    run_ok(dir.path(), &["new", "two"]);
    run_ok(dir.path(), &["new", "three"]);

    run_ok(dir.path(), &["update", "2", "TWO"]);

    let contents = std::fs::read_to_string(main_devlog(dir.path())).unwrap();
    assert!(contents.contains(": one"));
    assert!(contents.contains(": TWO"));
    assert!(contents.contains(": three"));
    assert!(!contents.contains(": two")); // replaced
}

#[test]
fn update_preserves_entry_number_and_date() {
    let dir = tempfile::tempdir().unwrap();
    let new_out = run_ok(dir.path(), &["new", "original"]);
    let orig_date = first_entry_date(&new_out);

    run_ok(dir.path(), &["update", "1", "updated"]);

    let list_out = run_ok(dir.path(), &["list"]);
    let line = list_out.lines().next().unwrap();
    assert!(line.starts_with("- 1 | "), "number preserved: {line}");
    assert!(line.contains(&orig_date), "date preserved: got {line}, orig_date={orig_date}");
}

#[test]
fn update_by_exact_date() {
    let dir = tempfile::tempdir().unwrap();
    let new_out = run_ok(dir.path(), &["new", "original"]);
    let orig_date = first_entry_date(&new_out);

    run_ok(dir.path(), &["update", &orig_date, "via date"]);
    let list = run_ok(dir.path(), &["list"]);
    assert!(list.contains(": via date"), "got: {list}");
}

#[test]
fn update_by_date_prefix_when_unique() {
    let dir = tempfile::tempdir().unwrap();
    let new_out = run_ok(dir.path(), &["new", "only entry today"]);
    let orig_date = first_entry_date(&new_out);
    let date_part = orig_date.split_once(' ').unwrap().0; // YYYY-MM-DD

    run_ok(dir.path(), &["update", date_part, "via prefix"]);
    let list = run_ok(dir.path(), &["list"]);
    assert!(list.contains(": via prefix"));
}

#[test]
fn update_nonexistent_number_errors() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "only one"]);
    let stderr = run_err(dir.path(), &["update", "99", "nope"]);
    assert!(stderr.contains("no entry with number 99"), "stderr: {stderr}");
}

#[test]
fn update_missing_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let stderr = run_err(dir.path(), &["update", "1", "nope"]);
    assert!(stderr.contains("devlog not found"), "stderr: {stderr}");
}

#[test]
fn update_invalid_id_errors() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "hi"]);
    let stderr = run_err(dir.path(), &["update", "not-a-number-or-date", "nope"]);
    assert!(stderr.contains("no entry matches id"), "stderr: {stderr}");
}

#[test]
fn update_section_entry() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "backend", "before"]);
    run_ok(dir.path(), &["update", "backend", "1", "after"]);
    let list = run_ok(dir.path(), &["list", "backend"]);
    assert!(list.contains(": after"));
    assert!(!list.contains(": before"));
}

#[test]
fn update_rejects_invalid_section_name() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _, stderr) = run(dir.path(), &["update", "Bad", "1", "x"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("invalid section name"), "stderr: {stderr}");
}

#[test]
fn update_preserves_prose_between_entries() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "one"]);
    run_ok(dir.path(), &["new", "two"]);

    // Inject prose between the two entries.
    let path = main_devlog(dir.path());
    let contents = std::fs::read_to_string(&path).unwrap();
    let injected = contents.replacen(
        "- 2 | ",
        "\n## A header added by the user\nSome prose here.\n\n- 2 | ",
        1,
    );
    std::fs::write(&path, &injected).unwrap();

    // Now update entry 1.
    run_ok(dir.path(), &["update", "1", "ONE"]);

    let after = std::fs::read_to_string(&path).unwrap();
    assert!(after.contains("## A header added by the user"));
    assert!(after.contains("Some prose here."));
    assert!(after.contains(": ONE"));
    assert!(after.contains(": two"));
}
