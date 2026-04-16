//! The [`DevlogServer`] — MCP server adapter exposing the devlogger
//! library through the Model Context Protocol.
//!
//! Each of devlogger's five CLI commands becomes an MCP tool:
//!
//! | Tool              | CLI equivalent                                |
//! |-------------------|-----------------------------------------------|
//! | `devlog_new`      | `devlogger new <section> <text>`              |
//! | `devlog_list`     | `devlogger list [<section>]`                  |
//! | `devlog_sections` | `devlogger sections`                          |
//! | `devlog_update`   | `devlogger update <section> <id> <text>`      |
//! | `devlog_read`     | `devlogger read <section> [<n>]`              |
//!
//! All tools accept an optional `base_dir` argument that overrides the
//! server's configured default directory on a per-call basis — the same
//! semantics as the CLI's `-f` flag.
//!
//! Errors from the underlying library (invalid section names, missing
//! files, parse failures, etc.) are surfaced as tool-level errors
//! (`CallToolResult::error(...)`) so an LLM client can read the message
//! and retry; only task-join / JSON serialization failures become
//! protocol-level errors.

use std::path::PathBuf;
use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo,
    },
    tool, tool_handler, tool_router,
};

use crate::commands::{cmd_list, cmd_list_all, cmd_new, cmd_read, cmd_sections, cmd_update};
use crate::mcp::args::{ListArgs, NewArgs, ReadArgs, SectionsArgs, UpdateArgs};
use crate::mcp::convert::{EntryJson, SectionEntriesJson, entries_to_json};

/// MCP server wrapping the devlogger library.
///
/// Cloneable so rmcp can share it across request tasks.  The
/// `default_base` is the directory that contains (or will contain) the
/// `DEVLOG/` folder when a tool call omits its own `base_dir` override.
#[derive(Clone)]
pub struct DevlogServer {
    default_base: Arc<PathBuf>,
    // The `#[tool_router]` macro drives this field through its generated
    // `call_tool` path; rustc's dead-code pass doesn't see that read.
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl DevlogServer {
    /// Build a new server with the given default base directory.
    pub fn new(default_base: PathBuf) -> Self {
        Self {
            default_base: Arc::new(default_base),
            tool_router: Self::tool_router(),
        }
    }

    /// The directory currently used when a tool call doesn't override it.
    pub fn default_base(&self) -> &std::path::Path {
        self.default_base.as_path()
    }

    /// Resolve the base directory for a single tool invocation.
    fn resolve_base(&self, override_: Option<&str>) -> PathBuf {
        match override_ {
            Some(s) if !s.is_empty() => PathBuf::from(s),
            _ => (*self.default_base).clone(),
        }
    }

    #[tool(
        name = "devlog_new",
        description = "Append a new entry to a section's devlog. Section names must match [a-z]+(-[a-z]+)*. \
                       Entry text must be single-line. Returns the canonical entry line plus structured fields \
                       (number, date, text)."
    )]
    pub async fn devlog_new(
        &self,
        Parameters(args): Parameters<NewArgs>,
    ) -> Result<CallToolResult, McpError> {
        let base = self.resolve_base(args.base_dir.as_deref());
        let section = args.section;
        let text = args.text;
        let result = tokio::task::spawn_blocking(move || cmd_new(&base, &section, &text))
            .await
            .map_err(join_error)?;
        match result {
            Ok(entry) => {
                let json: EntryJson = (&entry).into();
                success_with_json(vec![Content::text(entry.to_line())], &json)
            }
            Err(e) => Ok(tool_error(format_report(&e))),
        }
    }

    #[tool(
        name = "devlog_list",
        description = "List entries. With a section name, returns only that section's entries. Without one, \
                       returns every section's entries grouped by section (alphabetical). Structured result is \
                       either an array of entries or an array of {section, entries} objects; the text result is \
                       a human-readable summary."
    )]
    pub async fn devlog_list(
        &self,
        Parameters(args): Parameters<ListArgs>,
    ) -> Result<CallToolResult, McpError> {
        let base = self.resolve_base(args.base_dir.as_deref());
        match args.section {
            Some(section) => {
                let result = tokio::task::spawn_blocking(move || cmd_list(&base, &section))
                    .await
                    .map_err(join_error)?;
                match result {
                    Ok(entries) => {
                        let json = entries_to_json(&entries);
                        let summary = entries
                            .iter()
                            .map(|e| e.to_line())
                            .collect::<Vec<_>>()
                            .join("\n");
                        success_with_json(vec![Content::text(summary)], &json)
                    }
                    Err(e) => Ok(tool_error(format_report(&e))),
                }
            }
            None => {
                let result = tokio::task::spawn_blocking(move || cmd_list_all(&base))
                    .await
                    .map_err(join_error)?;
                match result {
                    Ok(groups) => {
                        let json: Vec<SectionEntriesJson> = groups
                            .iter()
                            .map(|(name, entries)| SectionEntriesJson {
                                section: name.clone(),
                                entries: entries_to_json(entries),
                            })
                            .collect();
                        let mut summary = String::new();
                        for (name, entries) in &groups {
                            for e in entries {
                                summary.push_str(&format!("[{name}] {}\n", e.to_line()));
                            }
                        }
                        // Trim the trailing newline for a tidier text view.
                        if summary.ends_with('\n') {
                            summary.pop();
                        }
                        success_with_json(vec![Content::text(summary)], &json)
                    }
                    Err(e) => Ok(tool_error(format_report(&e))),
                }
            }
        }
    }

    #[tool(
        name = "devlog_sections",
        description = "List all section names in alphabetical order. Empty array if no sections exist yet."
    )]
    pub async fn devlog_sections(
        &self,
        Parameters(args): Parameters<SectionsArgs>,
    ) -> Result<CallToolResult, McpError> {
        let base = self.resolve_base(args.base_dir.as_deref());
        let result = tokio::task::spawn_blocking(move || cmd_sections(&base))
            .await
            .map_err(join_error)?;
        match result {
            Ok(names) => {
                let text = names.join("\n");
                success_with_json(vec![Content::text(text)], &names)
            }
            Err(e) => Ok(tool_error(format_report(&e))),
        }
    }

    #[tool(
        name = "devlog_update",
        description = "Rewrite an existing entry's text. `id` is either the entry number (from devlog_list), \
                       an exact `YYYY-MM-DD HH:MM:SS` timestamp, or a unique date prefix. The entry's number \
                       and date are preserved; only the text changes."
    )]
    pub async fn devlog_update(
        &self,
        Parameters(args): Parameters<UpdateArgs>,
    ) -> Result<CallToolResult, McpError> {
        let base = self.resolve_base(args.base_dir.as_deref());
        let section = args.section;
        let id = args.id;
        let text = args.text;
        let result =
            tokio::task::spawn_blocking(move || cmd_update(&base, &section, &id, &text))
                .await
                .map_err(join_error)?;
        match result {
            Ok(entry) => {
                let json: EntryJson = (&entry).into();
                success_with_json(vec![Content::text(entry.to_line())], &json)
            }
            Err(e) => Ok(tool_error(format_report(&e))),
        }
    }

    #[tool(
        name = "devlog_read",
        description = "Read a section's devlog. With no `n`, returns the entire file verbatim (including any \
                       non-entry prose). With `n`, returns just the last `n` entry lines."
    )]
    pub async fn devlog_read(
        &self,
        Parameters(args): Parameters<ReadArgs>,
    ) -> Result<CallToolResult, McpError> {
        let base = self.resolve_base(args.base_dir.as_deref());
        let section = args.section;
        let n = args.n;
        let result = tokio::task::spawn_blocking(move || cmd_read(&base, &section, n))
            .await
            .map_err(join_error)?;
        match result {
            Ok(contents) => success_with_json(
                vec![Content::text(contents.clone())],
                &serde_json::json!({ "contents": contents }),
            ),
            Err(e) => Ok(tool_error(format_report(&e))),
        }
    }
}

#[tool_handler]
impl ServerHandler for DevlogServer {
    fn get_info(&self) -> ServerInfo {
        // ServerInfo and Implementation are `#[non_exhaustive]`, so the
        // struct-literal shorthand doesn't work from outside the crate —
        // mutate the default instead.
        let mut implementation = Implementation::default();
        implementation.name = "devlogger".into();
        implementation.version = env!("CARGO_PKG_VERSION").into();

        let mut info = ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info = implementation;
        info.instructions = Some(
            "Append-only markdown devlog. Sections live at \
             <base>/DEVLOG/<section>/<section>-devlog.md. Use devlog_sections to discover \
             sections, devlog_list to browse, devlog_new after implementing a change, \
             devlog_update to rewrite an entry's text, and devlog_read to dump a section. \
             All tools accept an optional `base_dir` to target a different project root."
                .into(),
        );
        info
    }
}

/// Map a `tokio::task::JoinError` into a protocol-level `McpError`.
fn join_error(e: tokio::task::JoinError) -> McpError {
    McpError::internal_error(format!("blocking task failed: {e}"), None)
}

/// Format an `eyre::Report` with its full cause chain, matching the CLI's
/// `{e:#}` error output.
fn format_report(e: &eyre::Report) -> String {
    format!("{e:#}")
}

/// Build a successful `CallToolResult` with both a text/content payload
/// and a structured JSON payload.  Clients that look at only one of
/// these (older ones tend to use `content`, newer ones `structured_content`)
/// get a sensible answer either way.
fn success_with_json<T: serde::Serialize>(
    content: Vec<Content>,
    value: &T,
) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_value(value)
        .map_err(|e| McpError::internal_error(format!("json serialize failed: {e}"), None))?;
    let mut result = CallToolResult::success(content);
    result.structured_content = Some(json);
    Ok(result)
}

/// Build a tool-level error result.  The LLM sees the message and can
/// adjust its next call (fix section name, pick a different id, etc.)
/// rather than getting a protocol-level failure that aborts the request.
fn tool_error(msg: String) -> CallToolResult {
    CallToolResult::error(vec![Content::text(msg)])
}
