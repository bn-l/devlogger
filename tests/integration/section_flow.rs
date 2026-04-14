use super::common::{run, run_ok, section_devlog};

#[test]
fn section_creates_expected_path() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "backend", "first"]);
    assert_eq!(
        section_devlog(dir.path(), "backend"),
        dir.path().join("DEVLOG/backend/backend-devlog.md")
    );
    assert!(section_devlog(dir.path(), "backend").exists());
}

#[test]
fn multiple_sections_are_isolated() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "backend", "b1"]);
    run_ok(dir.path(), &["new", "frontend", "f1"]);
    run_ok(dir.path(), &["new", "backend", "b2"]);

    let back = run_ok(dir.path(), &["list", "backend"]);
    let front = run_ok(dir.path(), &["list", "frontend"]);

    assert!(back.contains(": b1"));
    assert!(back.contains(": b2"));
    assert!(!back.contains(": f1"));
    assert!(!front.contains(": b1"));
    assert!(front.contains(": f1"));
    assert!(!front.contains(": b2"));
}

#[test]
fn hyphenated_section_names_work() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "api-v-two", "first"]);
    assert!(section_devlog(dir.path(), "api-v-two").exists());
    let out = run_ok(dir.path(), &["list", "api-v-two"]);
    assert!(out.contains(": first"));
}

#[test]
fn section_name_validation_errors_on_every_command() {
    let dir = tempfile::tempdir().unwrap();
    for subcmd in [
        vec!["new", "5", "x"],
        vec!["list", "5"],
        vec!["read", "BadName"],
        vec!["update", "Bad", "1", "x"],
    ] {
        let (code, _, stderr) = run(dir.path(), &subcmd);
        assert_ne!(code, 0, "expected failure for {subcmd:?}");
        assert!(
            stderr.contains("invalid section name"),
            "stderr for {subcmd:?}: {stderr}"
        );
    }
}
