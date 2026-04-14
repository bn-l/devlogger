//! The binary's `--help` and `--version` output should contain the
//! information a user needs, not just be blank placeholder text.

use super::common::{bin, run_ok};

fn run_captured(args: &[&str]) -> (i32, String) {
    let out = std::process::Command::new(bin())
        .args(args)
        .output()
        .expect("spawn");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr),
    )
}

#[test]
fn help_exits_zero_and_lists_all_subcommands() {
    let (code, text) = run_captured(&["--help"]);
    assert_eq!(code, 0, "text: {text}");
    for word in ["new", "list", "update", "read", "sections"] {
        assert!(text.contains(word), "help missing subcommand `{word}`: {text}");
    }
    assert!(text.contains("--file") || text.contains("-f"));
}

#[test]
fn version_exits_zero_and_has_nonempty_output() {
    let (code, text) = run_captured(&["--version"]);
    assert_eq!(code, 0);
    assert!(text.contains("devlogger"));
}

#[test]
fn short_help_flag_works_too() {
    let (code, text) = run_captured(&["-h"]);
    assert_eq!(code, 0);
    assert!(!text.is_empty());
}

#[test]
fn subcommand_help_describes_subcommand() {
    let (code, text) = run_captured(&["new", "--help"]);
    assert_eq!(code, 0);
    assert!(text.to_lowercase().contains("entry") || text.contains("section"));
}

#[test]
fn no_args_shows_error_and_nonzero_exit() {
    let dir = tempfile::tempdir().unwrap();
    // Not a help test exactly — but confirms missing-subcommand UX.
    let out = std::process::Command::new(bin())
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_ne!(out.status.code().unwrap_or(-1), 0);
    assert!(!out.stderr.is_empty());
}

// Make sure help/usage tests don't depend on an existing devlog.
#[test]
fn help_works_in_empty_directory() {
    let dir = tempfile::tempdir().unwrap();
    let out = std::process::Command::new(bin())
        .current_dir(dir.path())
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
}

// And make sure plain operations also still work after the help tests
// above (sanity: the binary isn't broken).
#[test]
fn binary_runs_correctly_after_help_invocations() {
    let dir = tempfile::tempdir().unwrap();
    run_ok(dir.path(), &["new", "sanity"]);
}
