use std::process::ExitCode;

use clap::Parser;
use eyre::Result;

use devlogger::cli::{Cli, Command};
use devlogger::commands::{cmd_list, cmd_new, cmd_read, cmd_sections, cmd_update};
use devlogger::section::validate_section_name;

/// Maximum terminal display width (in columns) for a single `list` row.
const LIST_LINE_MAX: usize = 80;

fn run(cli: Cli) -> Result<()> {
    let base = match cli.file {
        Some(p) => p,
        None => std::env::current_dir()?,
    };

    match cli.command {
        Command::New { args } => {
            let (section, text) = split_new_args(args)?;
            let entry = cmd_new(&base, section.as_deref(), &text)?;
            println!("{}", entry.to_line());
        }
        Command::List { args } => {
            let section = args.into_iter().next();
            if let Some(ref s) = section {
                validate_section_name(s)?;
            }
            let entries = cmd_list(&base, section.as_deref())?;
            for e in &entries {
                println!("{}", e.to_line_truncated(LIST_LINE_MAX));
            }
        }
        Command::Sections => {
            for name in cmd_sections(&base)? {
                println!("{name}");
            }
        }
        Command::Update { args } => {
            let (section, id, text) = split_update_args(args)?;
            let entry = cmd_update(&base, section.as_deref(), &id, &text)?;
            println!("{}", entry.to_line());
        }
        Command::Read { args } => {
            let (section, n) = split_read_args(args)?;
            let out = cmd_read(&base, section.as_deref(), n)?;
            print!("{out}");
        }
    }
    Ok(())
}

fn split_new_args(args: Vec<String>) -> Result<(Option<String>, String)> {
    // clap guarantees 1..=2
    let mut it = args.into_iter();
    let first = it.next().expect("clap enforces at least 1 arg");
    match it.next() {
        None => Ok((None, first)),
        Some(second) => {
            validate_section_name(&first)?;
            Ok((Some(first), second))
        }
    }
}

fn split_update_args(args: Vec<String>) -> Result<(Option<String>, String, String)> {
    // clap guarantees 2..=3
    let mut it = args.into_iter();
    let a = it.next().expect("clap enforces at least 2 args");
    let b = it.next().expect("clap enforces at least 2 args");
    match it.next() {
        None => Ok((None, a, b)),
        Some(c) => {
            validate_section_name(&a)?;
            Ok((Some(a), b, c))
        }
    }
}

fn split_read_args(args: Vec<String>) -> Result<(Option<String>, Option<usize>)> {
    let mut it = args.into_iter();
    match (it.next(), it.next()) {
        (None, _) => Ok((None, None)),
        (Some(a), None) => {
            if let Ok(n) = a.parse::<usize>() {
                Ok((None, Some(n)))
            } else {
                validate_section_name(&a)?;
                Ok((Some(a), None))
            }
        }
        (Some(a), Some(b)) => {
            validate_section_name(&a)?;
            let n: usize = b.parse().map_err(|_| {
                eyre::eyre!("`<n>` must be a non-negative integer, got `{b}`")
            })?;
            Ok((Some(a), Some(n)))
        }
    }
}

fn main() -> ExitCode {
    color_eyre::install().ok();
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("devlogger: {e:#}");
            ExitCode::FAILURE
        }
    }
}
