use super::common::{run, run_err, run_ok};

#[test]
fn read_missing_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let stderr = run_err(dir.path(), &["read", "main"]);
    assert!(stderr.contains("devlog not found"), "stderr: {stderr}");
}

#[test]
fn read_without_section_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let (code, _, stderr) = run(dir.path(), &["read"]);
    assert_ne!(code, 0);
    assert!(!stderr.is_empty());
}

#[test]
fn read_full_file() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "main", "one"]);
    run_ok(dir.path(), &["new", "main", "two"]);
    let out = run_ok(dir.path(), &["read", "main"]);
    assert!(out.contains(": one"));
    assert!(out.contains(": two"));
}

#[test]
fn read_last_n_returns_tail() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "main", "one"]);
    run_ok(dir.path(), &["new", "main", "two"]);
    run_ok(dir.path(), &["new", "main", "three"]);

    let out = run_ok(dir.path(), &["read", "main", "1"]);
    assert!(out.contains(": three"), "got: {out}");
    assert!(!out.contains(": one"), "got: {out}");
    assert!(!out.contains(": two"), "got: {out}");
}

#[test]
fn read_last_two() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "main", "one"]);
    run_ok(dir.path(), &["new", "main", "two"]);
    run_ok(dir.path(), &["new", "main", "three"]);

    let out = run_ok(dir.path(), &["read", "main", "2"]);
    assert!(!out.contains(": one"), "got: {out}");
    assert!(out.contains(": two"));
    assert!(out.contains(": three"));
}

#[test]
fn read_n_larger_than_count_returns_all() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "main", "only"]);
    let out = run_ok(dir.path(), &["read", "main", "42"]);
    assert!(out.contains(": only"));
}

#[test]
fn read_zero_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "main", "one"]);
    let out = run_ok(dir.path(), &["read", "main", "0"]);
    assert!(out.is_empty(), "got: {out:?}");
}

#[test]
fn read_section_full() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "backend", "one"]);
    run_ok(dir.path(), &["new", "backend", "two"]);
    let out = run_ok(dir.path(), &["read", "backend"]);
    assert!(out.contains(": one"));
    assert!(out.contains(": two"));
}

#[test]
fn read_section_last_n() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "backend", "one"]);
    run_ok(dir.path(), &["new", "backend", "two"]);
    run_ok(dir.path(), &["new", "backend", "three"]);
    let out = run_ok(dir.path(), &["read", "backend", "1"]);
    assert!(out.contains(": three"));
    assert!(!out.contains(": one"));
}
