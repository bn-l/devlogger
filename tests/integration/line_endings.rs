//! Devlogger must read either LF or CRLF files and write them back with
//! their original terminator preserved.

use super::common::{run_ok, section_devlog};

fn seed(path: &std::path::Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn count_crlf(bytes: &[u8]) -> usize {
    bytes.windows(2).filter(|w| w == b"\r\n").count()
}

fn count_bare_lf(bytes: &[u8]) -> usize {
    // Total LFs minus LFs that are part of CRLF.
    bytes.iter().filter(|&&b| b == b'\n').count() - count_crlf(bytes)
}

// ---- CRLF preservation ----

#[test]
fn crlf_file_preserved_after_update() {
    let dir = tempfile::tempdir().unwrap();
    let path = section_devlog(dir.path(), "main");
    let before = b"- 1 | 2026-04-14 10:00:00: one\r\n- 2 | 2026-04-14 11:00:00: two\r\n";
    seed(&path, before);

    run_ok(dir.path(), &["update", "main", "2", "two-updated"]);

    let after = std::fs::read(&path).unwrap();
    assert_eq!(
        count_crlf(&after),
        2,
        "should have 2 CRLF sequences: {after:?}"
    );
    assert_eq!(count_bare_lf(&after), 0, "no bare LFs allowed: {after:?}");
    assert!(String::from_utf8(after).unwrap().contains(": two-updated"));
}

#[test]
fn crlf_file_preserved_after_new() {
    let dir = tempfile::tempdir().unwrap();
    let path = section_devlog(dir.path(), "main");
    let before = b"- 1 | 2026-04-14 10:00:00: one\r\n";
    seed(&path, before);

    run_ok(dir.path(), &["new", "main", "two"]);

    let after = std::fs::read(&path).unwrap();
    assert_eq!(count_crlf(&after), 2, "one per entry: {after:?}");
    assert_eq!(count_bare_lf(&after), 0, "no bare LFs allowed: {after:?}");
}

#[test]
fn crlf_preserved_across_prose_in_update() {
    let dir = tempfile::tempdir().unwrap();
    let path = section_devlog(dir.path(), "main");
    let before =
        b"# Header\r\n\r\nSome prose.\r\n- 1 | 2026-04-14 10:00:00: one\r\n- 2 | 2026-04-14 11:00:00: two\r\n";
    seed(&path, before);

    run_ok(dir.path(), &["update", "main", "1", "ONE"]);

    let after = std::fs::read(&path).unwrap();
    assert_eq!(
        count_bare_lf(&after),
        0,
        "should remain pure CRLF: {after:?}"
    );
    let s = String::from_utf8(after).unwrap();
    assert!(s.contains("# Header"));
    assert!(s.contains("Some prose."));
    assert!(s.contains(": ONE"));
    assert!(s.contains(": two"));
}

// ---- LF preservation ----

#[test]
fn lf_file_stays_lf_after_update() {
    let dir = tempfile::tempdir().unwrap();
    let path = section_devlog(dir.path(), "main");
    let before = b"- 1 | 2026-04-14 10:00:00: one\n- 2 | 2026-04-14 11:00:00: two\n";
    seed(&path, before);

    run_ok(dir.path(), &["update", "main", "2", "two-updated"]);

    let after = std::fs::read(&path).unwrap();
    assert_eq!(count_crlf(&after), 0, "should have no CRLF: {after:?}");
    assert!(after.contains(&b'\n'));
}

#[test]
fn lf_file_stays_lf_after_new() {
    let dir = tempfile::tempdir().unwrap();
    let path = section_devlog(dir.path(), "main");
    seed(&path, b"- 1 | 2026-04-14 10:00:00: one\n");

    run_ok(dir.path(), &["new", "main", "two"]);

    let after = std::fs::read(&path).unwrap();
    assert_eq!(count_crlf(&after), 0);
    assert_eq!(count_bare_lf(&after), 2);
}

// ---- fresh file uses LF by default ----

#[test]
fn fresh_file_uses_lf_by_default() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "main", "first"]);
    let after = std::fs::read(section_devlog(dir.path(), "main")).unwrap();
    assert_eq!(count_crlf(&after), 0);
    assert!(after.ends_with(b"\n"));
}

// ---- read command tolerates both ----

#[test]
fn list_reads_crlf_file_correctly() {
    let dir = tempfile::tempdir().unwrap();
    let path = section_devlog(dir.path(), "main");
    seed(
        &path,
        b"- 1 | 2026-04-14 10:00:00: one\r\n- 2 | 2026-04-14 11:00:00: two\r\n",
    );

    let out = run_ok(dir.path(), &["list", "main"]);
    assert!(out.contains(": one"));
    assert!(out.contains(": two"));
}
