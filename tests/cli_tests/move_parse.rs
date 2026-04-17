use clap::Parser;
use devlogger::cli::{Cli, Command};

#[test]
fn move_with_three_args_is_from_id_to() {
    let cli = Cli::try_parse_from(["devlogger", "move", "backend", "2", "frontend"]).unwrap();
    match cli.command {
        Command::Move { args } => {
            assert_eq!(args, vec!["backend", "2", "frontend"]);
        }
        _ => panic!("expected Move"),
    }
}

#[test]
fn move_accepts_date_as_id() {
    let cli = Cli::try_parse_from([
        "devlogger",
        "move",
        "backend",
        "2026-04-14 11:02:37",
        "frontend",
    ])
    .unwrap();
    match cli.command {
        Command::Move { args } => {
            assert_eq!(args[0], "backend");
            assert_eq!(args[1], "2026-04-14 11:02:37");
            assert_eq!(args[2], "frontend");
        }
        _ => panic!(),
    }
}

#[test]
fn move_with_two_args_is_error() {
    assert!(Cli::try_parse_from(["devlogger", "move", "a", "b"]).is_err());
}

#[test]
fn move_with_one_arg_is_error() {
    assert!(Cli::try_parse_from(["devlogger", "move", "a"]).is_err());
}

#[test]
fn move_with_zero_args_is_error() {
    assert!(Cli::try_parse_from(["devlogger", "move"]).is_err());
}

#[test]
fn move_with_four_args_is_error() {
    assert!(Cli::try_parse_from(["devlogger", "move", "a", "b", "c", "d"]).is_err());
}
