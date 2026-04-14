use clap::Parser;
use devlogger::cli::{Cli, Command};

#[test]
fn sections_parses_without_args() {
    let cli = Cli::try_parse_from(["devlogger", "sections"]).unwrap();
    assert!(matches!(cli.command, Command::Sections));
}

#[test]
fn sections_rejects_positional_arg() {
    assert!(Cli::try_parse_from(["devlogger", "sections", "extra"]).is_err());
}

#[test]
fn sections_accepts_global_file_flag() {
    let cli = Cli::try_parse_from(["devlogger", "-f", "/tmp/x", "sections"]).unwrap();
    assert_eq!(cli.file.as_deref(), Some(std::path::Path::new("/tmp/x")));
    assert!(matches!(cli.command, Command::Sections));
}
