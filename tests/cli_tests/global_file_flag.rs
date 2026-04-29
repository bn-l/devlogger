use std::path::PathBuf;

use clap::Parser;
use devlogger::cli::{Cli, Command};

#[test]
fn file_flag_defaults_to_none() {
    let cli = Cli::try_parse_from(["devlogger", "list"]).unwrap();
    assert!(cli.file.is_none());
}

#[test]
fn short_flag_before_subcommand() {
    let cli = Cli::try_parse_from(["devlogger", "-f", "/tmp/proj", "list"]).unwrap();
    assert_eq!(cli.file, Some(PathBuf::from("/tmp/proj")));
    assert!(matches!(cli.command, Command::List { .. }));
}

#[test]
fn short_flag_after_subcommand() {
    // `global = true` allows the flag to appear after the subcommand too.
    let cli = Cli::try_parse_from(["devlogger", "list", "-f", "/tmp/proj"]).unwrap();
    assert_eq!(cli.file, Some(PathBuf::from("/tmp/proj")));
}

#[test]
fn short_flag_after_subcommand_with_positional() {
    let cli = Cli::try_parse_from(["devlogger", "new", "backend", "my entry", "-f", "/tmp/proj"])
        .unwrap();
    assert_eq!(cli.file, Some(PathBuf::from("/tmp/proj")));
    match cli.command {
        Command::New { args } => assert_eq!(args, vec!["backend", "my entry"]),
        _ => panic!(),
    }
}

#[test]
fn long_flag_is_also_global() {
    let cli = Cli::try_parse_from(["devlogger", "list", "--file", "/tmp/proj"]).unwrap();
    assert_eq!(cli.file, Some(PathBuf::from("/tmp/proj")));
}

#[test]
fn long_flag_before_subcommand() {
    let cli = Cli::try_parse_from(["devlogger", "--file", "/x", "read", "backend"]).unwrap();
    assert_eq!(cli.file, Some(PathBuf::from("/x")));
}
