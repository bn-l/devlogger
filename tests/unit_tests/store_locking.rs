//! Unit-level tests for the file-locking primitives.  These don't spawn
//! the binary — they drive the library API directly, which is cheaper
//! and pins the contract (lockfile path, lock exclusivity).

use std::thread;
use std::time::Duration;

use devlogger::section::{main_devlog_path, section_devlog_path};
use devlogger::store::{acquire_lock_for, lock_path_for};

#[test]
fn lock_path_is_sidecar_in_same_dir_for_main() {
    let dir = tempfile::tempdir().unwrap();
    let main = main_devlog_path(dir.path());
    let lp = lock_path_for(&main).unwrap();
    assert_eq!(lp, dir.path().join("DEVLOG").join(".devlogger.lock"));
}

#[test]
fn lock_path_is_sidecar_in_same_dir_for_section() {
    let dir = tempfile::tempdir().unwrap();
    let sp = section_devlog_path(dir.path(), "backend");
    let lp = lock_path_for(&sp).unwrap();
    assert_eq!(
        lp,
        dir.path()
            .join("DEVLOG")
            .join("backend")
            .join(".devlogger.lock")
    );
}

#[test]
fn acquire_lock_creates_lockfile_and_parent_dir() {
    let dir = tempfile::tempdir().unwrap();
    let main = main_devlog_path(dir.path());
    assert!(!main.parent().unwrap().exists());

    let _guard = acquire_lock_for(&main).unwrap();

    assert!(main.parent().unwrap().is_dir(), "parent dir should be created");
    assert!(
        lock_path_for(&main).unwrap().is_file(),
        "lockfile should be created"
    );
}

#[test]
fn second_acquire_blocks_until_first_released() {
    // Hold an exclusive lock, then in a background thread try to acquire
    // again.  The background acquire must not complete until we drop.
    let dir = tempfile::tempdir().unwrap();
    let main = main_devlog_path(dir.path());
    let first = acquire_lock_for(&main).unwrap();

    let path = main.clone();
    let handle = thread::spawn(move || {
        let start = std::time::Instant::now();
        let _second = acquire_lock_for(&path).unwrap();
        start.elapsed()
    });

    // Give the thread time to try; confirm it hasn't finished.
    thread::sleep(Duration::from_millis(150));
    assert!(!handle.is_finished(), "second acquire should still be blocked");

    drop(first);

    let elapsed = handle.join().expect("thread panicked");
    assert!(
        elapsed >= Duration::from_millis(100),
        "second acquire finished too fast (elapsed={elapsed:?}); lock may not have blocked"
    );
}
