use clap::Parser;
use devlogger::cli::{Cli, Command};

#[test]
fn list_without_args_is_main() {
    let cli = Cli::try_parse_from(["devlogger", "list"]).unwrap();
    match cli.command {
        Command::List { args } => assert!(args.is_empty()),
        _ => panic!("expected List"),
    }
}

#[test]
fn list_with_section_arg() {
    let cli = Cli::try_parse_from(["devlogger", "list", "backend"]).unwrap();
    match cli.command {
        Command::List { args } => assert_eq!(args, vec!["backend"]),
        _ => panic!("expected List"),
    }
}

#[test]
fn list_with_two_args_is_error() {
    assert!(Cli::try_parse_from(["devlogger", "list", "a", "b"]).is_err());
}
