//! Input argument structs for each MCP tool.  The doc comments on each
//! field become the JSON Schema descriptions exposed to MCP clients.
//!
//! Every tool accepts an optional `base_dir` that overrides the server's
//! default directory — same shape as the CLI's `-f` flag, so a single
//! server process can service multiple project roots if a client wants to.

use schemars::JsonSchema;
use serde::Deserialize;

/// Arguments for `devlog_new`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct NewArgs {
    /// Section name. Lowercase letters and single hyphens only, e.g.
    /// `parser`, `cli`, `store`. Must match `[a-z]+(-[a-z]+)*`.
    pub section: String,
    /// Entry text. Single line — no newlines or carriage returns.
    pub text: String,
    /// Optional override for the directory containing the `DEVLOG/` folder.
    /// Defaults to the server's configured base directory.
    #[serde(default)]
    pub base_dir: Option<String>,
}

/// Arguments for `devlog_list`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListArgs {
    /// Optional section name. When omitted, lists entries for every
    /// section grouped by section name (alphabetical).
    #[serde(default)]
    pub section: Option<String>,
    /// Optional override for the directory containing the `DEVLOG/` folder.
    #[serde(default)]
    pub base_dir: Option<String>,
}

/// Arguments for `devlog_sections`.
#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct SectionsArgs {
    /// Optional override for the directory containing the `DEVLOG/` folder.
    #[serde(default)]
    pub base_dir: Option<String>,
}

/// Arguments for `devlog_update`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateArgs {
    /// Section name (required).
    pub section: String,
    /// Entry id: either the entry number (from `devlog_list`), an exact
    /// `YYYY-MM-DD HH:MM:SS` timestamp, or a unique date prefix
    /// (e.g. `2026-04-14`).
    pub id: String,
    /// New entry text (replaces the existing text — number and date are
    /// preserved).
    pub text: String,
    /// Optional override for the directory containing the `DEVLOG/` folder.
    #[serde(default)]
    pub base_dir: Option<String>,
}

/// Arguments for `devlog_read`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadArgs {
    /// Section name (required).
    pub section: String,
    /// Optional count — return only the last `n` entry lines. When
    /// omitted, returns the full file verbatim (including prose lines).
    #[serde(default)]
    pub n: Option<usize>,
    /// Optional override for the directory containing the `DEVLOG/` folder.
    #[serde(default)]
    pub base_dir: Option<String>,
}
