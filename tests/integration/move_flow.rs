use super::common::{run, run_err, run_ok, section_devlog};

/// Seed a section's devlog directly on disk with deterministic dates so
/// the insertion-position logic is testable without racing `Local::now()`.
fn seed(base: &std::path::Path, section: &str, entries: &[(&str, &str)]) {
    let path = section_devlog(base, section);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut body = String::new();
    for (i, (date, text)) in entries.iter().enumerate() {
        body.push_str(&format!("- {} | {}: {}\n", i + 1, date, text));
    }
    std::fs::write(&path, body).unwrap();
}

fn entry_lines(path: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| l.starts_with("- "))
        .map(str::to_string)
        .collect()
}

#[test]
fn move_appends_newer_entry_to_end_of_dest() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), "from", &[("2026-04-15 09:00:00", "moving me")]);
    seed(
        dir.path(),
        "to",
        &[("2026-04-10 09:00:00", "a"), ("2026-04-12 09:00:00", "b")],
    );

    let out = run_ok(dir.path(), &["move", "from", "1", "to"]);
    assert!(
        out.starts_with("- 3 | "),
        "expected new number 3 in dest, got {out}"
    );

    let dst = entry_lines(&section_devlog(dir.path(), "to"));
    assert_eq!(dst.len(), 3);
    assert!(dst[0].starts_with("- 1 | 2026-04-10"));
    assert!(dst[1].starts_with("- 2 | 2026-04-12"));
    assert!(dst[2].starts_with("- 3 | 2026-04-15"));
    assert!(dst[2].ends_with(": moving me"));
}

#[test]
fn move_slots_entry_in_middle_by_date() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), "from", &[("2026-04-13 09:00:00", "mid")]);
    seed(
        dir.path(),
        "to",
        &[
            ("2026-04-10 09:00:00", "a"),
            ("2026-04-15 09:00:00", "b"),
            ("2026-04-20 09:00:00", "c"),
        ],
    );

    run_ok(dir.path(), &["move", "from", "1", "to"]);

    let dst = entry_lines(&section_devlog(dir.path(), "to"));
    assert_eq!(dst.len(), 4);
    assert!(dst[0].starts_with("- 1 | 2026-04-10"));
    assert!(dst[1].starts_with("- 2 | 2026-04-13"));
    assert!(dst[1].ends_with(": mid"));
    assert!(dst[2].starts_with("- 3 | 2026-04-15"));
    assert!(dst[3].starts_with("- 4 | 2026-04-20"));
}

#[test]
fn move_renumbers_source_after_removal() {
    let dir = tempfile::tempdir().unwrap();
    seed(
        dir.path(),
        "from",
        &[
            ("2026-04-10 09:00:00", "a"),
            ("2026-04-11 09:00:00", "b"),
            ("2026-04-12 09:00:00", "c"),
        ],
    );
    seed(dir.path(), "to", &[]);

    // Move the middle one.
    run_ok(dir.path(), &["move", "from", "2", "to"]);

    let src = entry_lines(&section_devlog(dir.path(), "from"));
    assert_eq!(src.len(), 2, "one entry moved out");
    assert!(src[0].starts_with("- 1 | 2026-04-10"));
    assert!(src[0].ends_with(": a"));
    assert!(src[1].starts_with("- 2 | 2026-04-12"));
    assert!(src[1].ends_with(": c"));
}

#[test]
fn move_to_new_section_creates_it() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), "from", &[("2026-04-10 09:00:00", "lone")]);

    run_ok(dir.path(), &["move", "from", "1", "fresh"]);

    let fresh = entry_lines(&section_devlog(dir.path(), "fresh"));
    assert_eq!(fresh.len(), 1);
    assert!(fresh[0].starts_with("- 1 | 2026-04-10"));
    assert!(fresh[0].ends_with(": lone"));

    let sections = run_ok(dir.path(), &["sections"]);
    assert!(sections.contains("fresh"));
}

#[test]
fn move_by_exact_date_resolves_target() {
    let dir = tempfile::tempdir().unwrap();
    seed(
        dir.path(),
        "from",
        &[
            ("2026-04-10 09:00:00", "wrong"),
            ("2026-04-11 14:33:22", "right"),
        ],
    );
    seed(dir.path(), "to", &[]);

    run_ok(dir.path(), &["move", "from", "2026-04-11 14:33:22", "to"]);

    let dst = entry_lines(&section_devlog(dir.path(), "to"));
    assert_eq!(dst.len(), 1);
    assert!(dst[0].ends_with(": right"));
}

#[test]
fn move_same_section_errors() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), "core", &[("2026-04-14 09:00:00", "x")]);

    let stderr = run_err(dir.path(), &["move", "core", "1", "core"]);
    assert!(stderr.contains("same section"), "stderr: {stderr}");
}

#[test]
fn move_missing_source_errors() {
    let dir = tempfile::tempdir().unwrap();
    let stderr = run_err(dir.path(), &["move", "nope", "1", "elsewhere"]);
    assert!(stderr.contains("not found"), "stderr: {stderr}");
}

#[test]
fn move_unknown_id_errors() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), "from", &[("2026-04-14 09:00:00", "only")]);
    let stderr = run_err(dir.path(), &["move", "from", "99", "to"]);
    assert!(stderr.contains("no entry"), "stderr: {stderr}");
}

#[test]
fn move_rejects_invalid_dest_section() {
    let dir = tempfile::tempdir().unwrap();
    seed(dir.path(), "from", &[("2026-04-14 09:00:00", "x")]);
    let (code, _, stderr) = run(dir.path(), &["move", "from", "1", "Bad"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("invalid section"), "stderr: {stderr}");
}

#[test]
fn move_preserves_prose_in_both_files() {
    let dir = tempfile::tempdir().unwrap();
    let from_path = section_devlog(dir.path(), "from");
    let to_path = section_devlog(dir.path(), "to");
    std::fs::create_dir_all(from_path.parent().unwrap()).unwrap();
    std::fs::create_dir_all(to_path.parent().unwrap()).unwrap();

    std::fs::write(
        &from_path,
        "# from section\n\n\
         - 1 | 2026-04-10 09:00:00: keep me\n\
         \n\
         ## subheading\n\
         - 2 | 2026-04-14 09:00:00: moving\n",
    )
    .unwrap();
    std::fs::write(
        &to_path,
        "# to section\n\n\
         - 1 | 2026-04-01 09:00:00: existing\n\n\
         trailing note\n",
    )
    .unwrap();

    run_ok(dir.path(), &["move", "from", "2", "to"]);

    let from_after = std::fs::read_to_string(&from_path).unwrap();
    assert!(from_after.contains("# from section"));
    assert!(from_after.contains("## subheading"));
    assert!(from_after.contains(": keep me"));
    assert!(!from_after.contains(": moving"));

    let to_after = std::fs::read_to_string(&to_path).unwrap();
    assert!(to_after.contains("# to section"));
    assert!(to_after.contains("trailing note"));
    assert!(to_after.contains(": moving"));
}
