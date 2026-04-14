use clap::Parser;
use devlogger::cli::{Cli, Command};

#[test]
fn read_with_no_args_is_error() {
    // Section is now required.
    assert!(Cli::try_parse_from(["devlogger", "read"]).is_err());
}

#[test]
fn read_with_section_arg() {
    let cli = Cli::try_parse_from(["devlogger", "read", "backend"]).unwrap();
    match cli.command {
        Command::Read { args } => assert_eq!(args, vec!["backend"]),
        _ => panic!(),
    }
}

#[test]
fn read_with_section_and_count() {
    let cli = Cli::try_parse_from(["devlogger", "read", "backend", "3"]).unwrap();
    match cli.command {
        Command::Read { args } => assert_eq!(args, vec!["backend", "3"]),
        _ => panic!(),
    }
}

#[test]
fn read_with_three_args_is_error() {
    assert!(Cli::try_parse_from(["devlogger", "read", "a", "b", "c"]).is_err());
}
