//! Clap definitions.  Each subcommand takes a single `Vec<String>` of
//! positional args with tight `num_args` bounds; `main.rs` dispatches
//! based on `args.len()`.  This keeps the ergonomic shape the user asked
//! for — `new [<section>] <entry>` etc. — without fighting clap's
//! optional-before-required positional semantics.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "devlogger",
    version,
    about = "Append-only markdown devlog CLI",
    long_about = "Manage a markdown devlog in <dir>/DEVLOG/.\n\n\
                  The main log lives at <dir>/DEVLOG/main-devlog.md.\n\
                  Named sections live at <dir>/DEVLOG/<section>/<section>-devlog.md.\n\n\
                  Section names must match [a-z]+(-[a-z]+)* — lowercase letters\n\
                  and hyphens only.\n\n\
                  Multi-word entries must be quoted on the command line."
)]
pub struct Cli {
    /// Directory containing (or to contain) the DEVLOG folder.
    /// Defaults to the current working directory.  May appear before or
    /// after the subcommand.
    #[arg(short = 'f', long = "file", global = true)]
    pub file: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Add a new entry.  `devlogger new [<section>] <entry>`
    New {
        #[arg(required = true, num_args = 1..=2)]
        args: Vec<String>,
    },

    /// List entries with their canonical numbers.  `devlogger list [<section>]`
    List {
        #[arg(num_args = 0..=1)]
        args: Vec<String>,
    },

    /// Update an entry's text.  `devlogger update [<section>] <id> <entry>`
    /// where <id> is the entry number or date shown by `list`.
    Update {
        #[arg(required = true, num_args = 2..=3)]
        args: Vec<String>,
    },

    /// Read the devlog.  `devlogger read [<section>] [<n>]`
    /// With no <n>, prints the whole file.  With <n>, prints the last
    /// <n> entry lines.
    Read {
        #[arg(num_args = 0..=2)]
        args: Vec<String>,
    },
}
