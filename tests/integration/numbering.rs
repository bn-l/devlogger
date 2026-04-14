//! Canonical-number semantics: sequential starting at 1, increases
//! monotonically, survives user hand-edits that introduce gaps, survives
//! updates (number is preserved through `update`).

use super::common::{main_devlog, run_ok};

fn seed(path: &std::path::Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn first_entry_in_empty_file_gets_number_one() {
    let dir = tempfile::tempdir().unwrap();
    let out = run_ok(dir.path(), &["new", "hello"]);
    assert!(out.starts_with("- 1 | "), "got: {out}");
}

#[test]
fn next_new_continues_past_hand_edited_gap() {
    // User has hand-edited entry numbers to have a gap — we still use
    // max(existing)+1, not "first available".
    let dir = tempfile::tempdir().unwrap();
    seed(
        &main_devlog(dir.path()),
        b"- 1 | 2026-04-14 10:00:00: a\n- 7 | 2026-04-14 11:00:00: g\n",
    );
    let out = run_ok(dir.path(), &["new", "fresh"]);
    assert!(out.starts_with("- 8 | "), "got: {out}");
}

#[test]
fn new_after_large_existing_number_continues_correctly() {
    let dir = tempfile::tempdir().unwrap();
    seed(
        &main_devlog(dir.path()),
        b"- 4294967294 | 2026-04-14 10:00:00: almost-max\n",
    );
    let out = run_ok(dir.path(), &["new", "at-max"]);
    assert!(out.starts_with("- 4294967295 | "), "got: {out}");
}

#[test]
fn update_does_not_shift_subsequent_numbers() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "one"]);
    run_ok(dir.path(), &["new", "two"]);
    run_ok(dir.path(), &["new", "three"]);

    run_ok(dir.path(), &["update", "1", "ONE-REVISED"]);

    let list = run_ok(dir.path(), &["list"]);
    let lines: Vec<&str> = list.lines().collect();
    assert!(lines[0].starts_with("- 1 | "));
    assert!(lines[1].starts_with("- 2 | "));
    assert!(lines[2].starts_with("- 3 | "));
}

#[test]
fn numbering_is_per_section_not_global() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "main-a"]);
    run_ok(dir.path(), &["new", "backend", "back-a"]);
    run_ok(dir.path(), &["new", "frontend", "front-a"]);
    run_ok(dir.path(), &["new", "backend", "back-b"]);

    let main = run_ok(dir.path(), &["list"]);
    let back = run_ok(dir.path(), &["list", "backend"]);
    let front = run_ok(dir.path(), &["list", "frontend"]);

    assert!(main.lines().next().unwrap().starts_with("- 1 | "));
    assert!(back.lines().next().unwrap().starts_with("- 1 | "));
    assert!(back.lines().nth(1).unwrap().starts_with("- 2 | "));
    assert!(front.lines().next().unwrap().starts_with("- 1 | "));
}

#[test]
fn update_preserves_number_even_for_hand_edited_non_sequential_file() {
    let dir = tempfile::tempdir().unwrap();
    seed(
        &main_devlog(dir.path()),
        b"- 3 | 2026-04-14 10:00:00: a\n- 42 | 2026-04-14 11:00:00: b\n",
    );

    run_ok(dir.path(), &["update", "42", "b-revised"]);

    let after = std::fs::read_to_string(main_devlog(dir.path())).unwrap();
    assert!(after.contains("- 3 | "), "entry 3 should still be 3");
    assert!(after.contains("- 42 | "), "entry 42 should still be 42");
    assert!(after.contains(": b-revised"));
    assert!(!after.contains(": b\n"));
}
