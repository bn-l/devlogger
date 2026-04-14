use clap::Parser;
use devlogger::cli::{Cli, Command};

#[test]
fn read_with_no_args() {
    let cli = Cli::try_parse_from(["devlogger", "read"]).unwrap();
    match cli.command {
        Command::Read { args } => assert!(args.is_empty()),
        _ => panic!("expected Read"),
    }
}

#[test]
fn read_with_numeric_arg() {
    let cli = Cli::try_parse_from(["devlogger", "read", "5"]).unwrap();
    match cli.command {
        Command::Read { args } => assert_eq!(args, vec!["5"]),
        _ => panic!(),
    }
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
