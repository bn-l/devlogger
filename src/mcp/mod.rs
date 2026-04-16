//! MCP (Model Context Protocol) server bindings for devlogger.
//!
//! This module adapts the library's command handlers into MCP tools so an
//! LLM agent can interact with a devlog the same way the CLI does.  See
//! [`server::DevlogServer`] for the main entry point.

pub mod args;
pub mod convert;
pub mod server;

pub use args::{ListArgs, NewArgs, ReadArgs, SectionsArgs, UpdateArgs};
pub use convert::{EntryJson, SectionEntriesJson, entries_to_json};
pub use server::DevlogServer;
