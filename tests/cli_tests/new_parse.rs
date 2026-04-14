use clap::Parser;
use devlogger::cli::{Cli, Command};

#[test]
fn new_with_two_args_is_section_and_entry() {
    let cli = Cli::try_parse_from(["devlogger", "new", "backend", "api work"]).unwrap();
    match cli.command {
        Command::New { args } => {
            assert_eq!(args, vec!["backend", "api work"]);
        }
        _ => panic!("expected New"),
    }
}

#[test]
fn new_with_one_arg_is_error() {
    // Section is now required: a single arg no longer means "entry only".
    assert!(Cli::try_parse_from(["devlogger", "new", "my entry"]).is_err());
}

#[test]
fn new_with_zero_args_is_error() {
    assert!(Cli::try_parse_from(["devlogger", "new"]).is_err());
}

#[test]
fn new_with_three_args_is_error() {
    assert!(Cli::try_parse_from(["devlogger", "new", "a", "b", "c"]).is_err());
}

#[test]
fn new_preserves_entry_whitespace_inside_quotes() {
    // On the CLI this would be passed via shell quoting; the argv entry
    // retains internal whitespace.
    let cli =
        Cli::try_parse_from(["devlogger", "new", "backend", "  spaced  text  "]).unwrap();
    match cli.command {
        Command::New { args } => assert_eq!(args, vec!["backend", "  spaced  text  "]),
        _ => panic!(),
    }
}
