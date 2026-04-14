//! Clap definitions.  `new`, `update`, `read` take a required section name
//! as their first positional arg; `list` takes an optional section (without
//! one, it lists every section's entries grouped by section).  `main.rs`
//! dispatches the positional `Vec<String>` into these shapes.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "devlogger",
    version,
    about = "Append-only markdown devlog CLI",
    long_about = "Manage a markdown devlog in <dir>/DEVLOG/.\n\n\
                  Sections live at <dir>/DEVLOG/<section>/<section>-devlog.md.\n\
                  Every entry belongs to a section — `new`, `update`, and\n\
                  `read` require a section name; `list` takes one optionally\n\
                  (without one, it prints every section's entries grouped by\n\
                  section).\n\n\
                  Section names must match [a-z]+(-[a-z]+)* — lowercase letters\n\
                  and hyphens only.\n\n\
                  `list` prints each entry truncated to 80 terminal columns,\n\
                  ending with ` (...N more)` when content was elided (N is the\n\
                  number of elided characters).  `sections` prints every\n\
                  section name, one per line.\n\n\
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
    /// Add a new entry.  `devlogger new <section> <entry>`
    New {
        #[arg(required = true, num_args = 2)]
        args: Vec<String>,
    },

    /// List entries with their canonical numbers.  `devlogger list [<section>]`
    ///
    /// With no section, prints every section's entries with a
    /// `[<section>] ` prefix on each line.  With a section, prints just
    /// that section's entries with no prefix.  Each row is truncated to
    /// 80 terminal columns (prefix included); elided entries end with
    /// ` (...N more)` where N is the number of elided characters.  Wide
    /// glyphs (CJK, most emoji) count as two columns.
    List {
        #[arg(num_args = 0..=1)]
        args: Vec<String>,
    },

    /// List all section names, one per line.  `devlogger sections`
    Sections,

    /// Update an entry's text.  `devlogger update <section> <id> <entry>`
    /// where <id> is the entry number or date shown by `list`.
    Update {
        #[arg(required = true, num_args = 3)]
        args: Vec<String>,
    },

    /// Read the devlog.  `devlogger read <section> [<n>]`
    /// With no <n>, prints the whole file.  With <n>, prints the last
    /// <n> entry lines.
    Read {
        #[arg(required = true, num_args = 1..=2)]
        args: Vec<String>,
    },
}
