//! Concurrency correctness: parallel `new` invocations must each get a
//! unique number and a full, unmerged line.  Before the lock + single
//! write_all landed, these tests reproduced duplicate numbers and lines
//! that ran into each other at byte level.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::thread;

use super::common::{bin, count_entries, section_devlog};

fn spawn_many_new(base: PathBuf, count: usize, section: &str) {
    let base = Arc::new(base);
    let section = Arc::new(section.to_string());
    let mut handles = Vec::with_capacity(count);
    for i in 0..count {
        let base = Arc::clone(&base);
        let section = Arc::clone(&section);
        handles.push(thread::spawn(move || {
            let mut cmd = Command::new(bin());
            cmd.arg("-f").arg(base.as_path()).arg("new");
            cmd.arg(section.as_str());
            cmd.arg(format!("entry {i}"));
            let out = cmd.output().expect("spawn devlogger");
            assert!(
                out.status.success(),
                "child {i} failed: stderr={}",
                String::from_utf8_lossy(&out.stderr)
            );
        }));
    }
    for h in handles {
        h.join().expect("child thread panicked");
    }
}

fn assert_distinct_numbers_one_through(path: &std::path::Path, n: usize) {
    let contents = std::fs::read_to_string(path).unwrap();
    let mut numbers: Vec<u32> = contents
        .lines()
        .filter_map(|l| l.strip_prefix("- "))
        .filter_map(|r| r.split_once(" | "))
        .filter_map(|(num, _)| num.trim().parse::<u32>().ok())
        .collect();
    numbers.sort_unstable();
    let expected: Vec<u32> = (1..=n as u32).collect();
    assert_eq!(numbers, expected, "file:\n{contents}");
}

fn assert_no_merged_lines(path: &std::path::Path) {
    let contents = std::fs::read_to_string(path).unwrap();
    for (i, line) in contents.lines().enumerate() {
        // Every non-empty line in the file should either be prose (not our
        // case here) or a SINGLE entry — never two entries concatenated.
        let dash_count = line
            .matches("- ")
            .filter(|_| line.starts_with("- "))
            .count()
            + line.matches(" - ").count();
        // A merged entry would contain "- N | " more than once.
        let entry_prefix_count = line.matches(" | ").count();
        assert!(
            entry_prefix_count <= 1,
            "line {i} contains {entry_prefix_count} entries merged together: {line:?}"
        );
        let _ = dash_count;
    }
}

#[test]
fn twenty_parallel_new_produces_twenty_unique_numbered_entries() {
    let dir = tempfile::tempdir().unwrap();
    spawn_many_new(dir.path().to_path_buf(), 20, "main");

    let path = section_devlog(dir.path(), "main");
    assert_eq!(count_entries(&path), 20, "expected 20 entry lines");
    assert_distinct_numbers_one_through(&path, 20);
    assert_no_merged_lines(&path);
}

#[test]
fn ten_parallel_new_on_a_different_section_also_safe() {
    let dir = tempfile::tempdir().unwrap();
    spawn_many_new(dir.path().to_path_buf(), 10, "backend");

    let path = section_devlog(dir.path(), "backend");
    assert_eq!(count_entries(&path), 10);
    assert_distinct_numbers_one_through(&path, 10);
    assert_no_merged_lines(&path);
}

#[test]
fn parallel_writers_to_different_sections_do_not_corrupt_each_other() {
    // Parallel writers to two sections use different lockfiles.  They
    // should both complete cleanly; each log ends up with its own
    // contiguous 1..N numbering.
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().to_path_buf();

    let b1 = base.clone();
    let t_a = thread::spawn(move || spawn_many_new(b1, 8, "alpha"));
    let b2 = base.clone();
    let t_b = thread::spawn(move || spawn_many_new(b2, 8, "backend"));

    t_a.join().unwrap();
    t_b.join().unwrap();

    assert_distinct_numbers_one_through(&section_devlog(dir.path(), "alpha"), 8);
    assert_distinct_numbers_one_through(&section_devlog(dir.path(), "backend"), 8);
}

#[test]
fn concurrent_updates_do_not_lose_writes() {
    // Seed N entries, then fire N concurrent `update` processes each
    // touching a distinct entry.  All updates must land.
    let dir = tempfile::tempdir().unwrap();
    let n = 10;
    for i in 0..n {
        let s = std::process::Command::new(bin())
            .arg("-f")
            .arg(dir.path())
            .arg("new")
            .arg("main")
            .arg(format!("initial {i}"))
            .output()
            .unwrap();
        assert!(s.status.success());
    }

    let base = Arc::new(dir.path().to_path_buf());
    let mut handles = Vec::with_capacity(n);
    for i in 0..n {
        let base = Arc::clone(&base);
        handles.push(thread::spawn(move || {
            let num = (i + 1).to_string();
            let out = std::process::Command::new(bin())
                .arg("-f")
                .arg(base.as_path())
                .arg("update")
                .arg("main")
                .arg(&num)
                .arg(format!("updated {i}"))
                .output()
                .expect("spawn");
            assert!(
                out.status.success(),
                "update {i} failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    let contents = std::fs::read_to_string(section_devlog(dir.path(), "main")).unwrap();
    for i in 0..n {
        let expected = format!(": updated {i}");
        assert!(
            contents.contains(&expected),
            "missing update {i}: file:\n{contents}"
        );
    }
}

#[test]
fn section_gets_its_own_lockfile() {
    let dir = tempfile::tempdir().unwrap();
    super::common::run_ok(dir.path(), &["new", "backend", "x"]);
    let section_lock = dir
        .path()
        .join("DEVLOG")
        .join("backend")
        .join(".devlogger.lock");
    assert!(section_lock.is_file());
    // A second, unrelated section must get its own lockfile, not share
    // one at the DEVLOG root.
    let root_lock = dir.path().join("DEVLOG").join(".devlogger.lock");
    assert!(
        !root_lock.exists(),
        "no root-level lockfile should be created"
    );
}
