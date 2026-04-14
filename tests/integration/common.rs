//! Shared helpers for integration tests.  Each test gets its own
//! `tempfile::TempDir` as the `-f` base, so the DEVLOG tree lives in
//! isolation.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Full path to the compiled `devlogger` binary, provided by Cargo.
pub fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_devlogger"))
}

/// Run devlogger with `-f <base>` prepended, plus the given args.  Returns
/// (exit code, stdout, stderr).
pub fn run(base: &Path, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(bin());
    cmd.arg("-f").arg(base);
    for a in args {
        cmd.arg(a);
    }
    let Output {
        status,
        stdout,
        stderr,
    } = cmd.output().expect("failed to spawn devlogger");
    (
        status.code().unwrap_or(-1),
        String::from_utf8_lossy(&stdout).into_owned(),
        String::from_utf8_lossy(&stderr).into_owned(),
    )
}

/// Assert a successful run, returning stdout.  Panics with both streams
/// on failure for easy debugging.
pub fn run_ok(base: &Path, args: &[&str]) -> String {
    let (code, stdout, stderr) = run(base, args);
    assert_eq!(
        code, 0,
        "expected success; args={args:?}; stdout={stdout}; stderr={stderr}"
    );
    stdout
}

/// Assert a failing run, returning stderr.
pub fn run_err(base: &Path, args: &[&str]) -> String {
    let (code, stdout, stderr) = run(base, args);
    assert_ne!(
        code, 0,
        "expected failure; args={args:?}; stdout={stdout}; stderr={stderr}"
    );
    stderr
}

pub fn section_devlog(base: &Path, section: &str) -> PathBuf {
    base.join("DEVLOG")
        .join(section)
        .join(format!("{section}-devlog.md"))
}

/// Count entry-shaped lines (`- N | ...`) in a file.
pub fn count_entries(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| l.starts_with("- "))
        .count()
}
