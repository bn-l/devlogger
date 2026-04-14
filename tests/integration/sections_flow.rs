use super::common::{run_ok, section_devlog};
use std::fs;

#[test]
fn sections_on_empty_project_prints_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let out = run_ok(dir.path(), &["sections"]);
    assert!(out.is_empty(), "expected empty output; got: {out:?}");
}

#[test]
fn sections_prints_created_sections_alphabetically() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "zeta", "z"]);
    run_ok(dir.path(), &["new", "alpha", "a"]);
    run_ok(dir.path(), &["new", "mango", "m"]);

    let out = run_ok(dir.path(), &["sections"]);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines, vec!["alpha", "mango", "zeta"]);
}

#[test]
fn sections_ignores_stray_directories() {
    let dir = tempfile::tempdir().unwrap();
    // Create a real section plus a stray subdir that looks section-y
    // but has no matching devlog file, and one with an invalid name.
    run_ok(dir.path(), &["new", "backend", "x"]);
    fs::create_dir_all(dir.path().join("DEVLOG/orphan")).unwrap();
    fs::create_dir_all(dir.path().join("DEVLOG/Invalid_Name")).unwrap();

    let out = run_ok(dir.path(), &["sections"]);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines, vec!["backend"]);
}

#[test]
fn sections_lists_multiple_real_sections() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "backend", "b1"]);
    run_ok(dir.path(), &["new", "frontend", "f1"]);
    run_ok(dir.path(), &["new", "api-v-two", "a1"]);

    assert!(section_devlog(dir.path(), "backend").exists());
    assert!(section_devlog(dir.path(), "frontend").exists());
    assert!(section_devlog(dir.path(), "api-v-two").exists());

    let out = run_ok(dir.path(), &["sections"]);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines, vec!["api-v-two", "backend", "frontend"]);
}

#[test]
fn sections_reflects_added_sections_across_invocations() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "backend", "b1"]);
    let out1 = run_ok(dir.path(), &["sections"]);
    assert_eq!(out1.lines().collect::<Vec<_>>(), vec!["backend"]);

    run_ok(dir.path(), &["new", "frontend", "f1"]);
    let out2 = run_ok(dir.path(), &["sections"]);
    assert_eq!(out2.lines().collect::<Vec<_>>(), vec!["backend", "frontend"]);
}
