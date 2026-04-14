use clap::Parser;
use devlogger::cli::Cli;

#[test]
fn help_flag_exits_with_help_error() {
    // clap returns a special "error" kind for --help that carries the
    // help text; we assert the error kind rather than exit behaviour.
    let err = Cli::try_parse_from(["devlogger", "--help"]).unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
}

#[test]
fn version_flag_exits_with_version_error() {
    let err = Cli::try_parse_from(["devlogger", "--version"]).unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
}

#[test]
fn no_subcommand_is_error() {
    let err = Cli::try_parse_from(["devlogger"]).unwrap_err();
    assert_ne!(err.kind(), clap::error::ErrorKind::DisplayHelp);
}

#[test]
fn unknown_subcommand_is_error() {
    assert!(Cli::try_parse_from(["devlogger", "frobnicate"]).is_err());
}
