use clap::Parser;
use devlogger::cli::{Cli, Command};

#[test]
fn update_with_two_args_is_id_and_entry() {
    let cli = Cli::try_parse_from(["devlogger", "update", "1", "fixed it"]).unwrap();
    match cli.command {
        Command::Update { args } => assert_eq!(args, vec!["1", "fixed it"]),
        _ => panic!("expected Update"),
    }
}

#[test]
fn update_with_three_args_is_section_id_entry() {
    let cli = Cli::try_parse_from(["devlogger", "update", "backend", "2", "refactor"]).unwrap();
    match cli.command {
        Command::Update { args } => {
            assert_eq!(args, vec!["backend", "2", "refactor"]);
        }
        _ => panic!("expected Update"),
    }
}

#[test]
fn update_accepts_date_as_id() {
    let cli = Cli::try_parse_from([
        "devlogger",
        "update",
        "2026-04-14 11:02:37",
        "revised text",
    ])
    .unwrap();
    match cli.command {
        Command::Update { args } => {
            assert_eq!(args[0], "2026-04-14 11:02:37");
            assert_eq!(args[1], "revised text");
        }
        _ => panic!(),
    }
}

#[test]
fn update_with_one_arg_is_error() {
    assert!(Cli::try_parse_from(["devlogger", "update", "1"]).is_err());
}

#[test]
fn update_with_zero_args_is_error() {
    assert!(Cli::try_parse_from(["devlogger", "update"]).is_err());
}

#[test]
fn update_with_four_args_is_error() {
    assert!(Cli::try_parse_from(["devlogger", "update", "a", "b", "c", "d"]).is_err());
}
