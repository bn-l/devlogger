use std::process::ExitCode;

use clap::Parser;
use eyre::Result;

use devlogger::cli::{Cli, Command};
use devlogger::commands::{
    cmd_list, cmd_list_all, cmd_move, cmd_new, cmd_read, cmd_sections, cmd_update,
};
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
            let entry = cmd_new(&base, &section, &text)?;
            println!("{}", entry.to_line());
        }
        Command::List { args } => match args.into_iter().next() {
            Some(section) => {
                validate_section_name(&section)?;
                let entries = cmd_list(&base, &section)?;
                for e in &entries {
                    println!("{}", e.to_line_truncated(LIST_LINE_MAX));
                }
            }
            None => {
                let groups = cmd_list_all(&base)?;
                for (name, entries) in groups {
                    // Section names are ASCII, so byte length = display width.
                    let prefix = format!("[{name}] ");
                    let budget = LIST_LINE_MAX.saturating_sub(prefix.len());
                    for e in &entries {
                        println!("{prefix}{}", e.to_line_truncated(budget));
                    }
                }
            }
        },
        Command::Sections => {
            for name in cmd_sections(&base)? {
                println!("{name}");
            }
        }
        Command::Update { args } => {
            let (section, id, text) = split_update_args(args)?;
            let entry = cmd_update(&base, &section, &id, &text)?;
            println!("{}", entry.to_line());
        }
        Command::Read { args } => {
            let (section, n) = split_read_args(args)?;
            let out = cmd_read(&base, &section, n)?;
            print!("{out}");
        }
        Command::Move { args } => {
            let (from, id, to) = split_move_args(args)?;
            let entry = cmd_move(&base, &from, &id, &to)?;
            println!("{}", entry.to_line());
        }
    }
    Ok(())
}

fn split_new_args(args: Vec<String>) -> Result<(String, String)> {
    // clap guarantees exactly 2
    let mut it = args.into_iter();
    let section = it.next().expect("clap enforces exactly 2 args");
    let text = it.next().expect("clap enforces exactly 2 args");
    validate_section_name(&section)?;
    Ok((section, text))
}

fn split_update_args(args: Vec<String>) -> Result<(String, String, String)> {
    // clap guarantees exactly 3
    let mut it = args.into_iter();
    let section = it.next().expect("clap enforces exactly 3 args");
    let id = it.next().expect("clap enforces exactly 3 args");
    let text = it.next().expect("clap enforces exactly 3 args");
    validate_section_name(&section)?;
    Ok((section, id, text))
}

fn split_move_args(args: Vec<String>) -> Result<(String, String, String)> {
    // clap guarantees exactly 3
    let mut it = args.into_iter();
    let from = it.next().expect("clap enforces exactly 3 args");
    let id = it.next().expect("clap enforces exactly 3 args");
    let to = it.next().expect("clap enforces exactly 3 args");
    validate_section_name(&from)?;
    validate_section_name(&to)?;
    Ok((from, id, to))
}

fn split_read_args(args: Vec<String>) -> Result<(String, Option<usize>)> {
    // clap guarantees 1..=2
    let mut it = args.into_iter();
    let section = it.next().expect("clap enforces at least 1 arg");
    validate_section_name(&section)?;
    match it.next() {
        None => Ok((section, None)),
        Some(b) => {
            let n: usize = b
                .parse()
                .map_err(|_| eyre::eyre!("`<n>` must be a non-negative integer, got `{b}`"))?;
            Ok((section, Some(n)))
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
