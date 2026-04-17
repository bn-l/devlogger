//! The [`DevlogServer`] — MCP server adapter exposing the devlogger
//! library through the Model Context Protocol.
//!
//! Each of devlogger's CLI commands becomes an MCP tool:
//!
//! | Tool              | CLI equivalent                                |
//! |-------------------|-----------------------------------------------|
//! | `devlog_new`      | `devlogger new <section> <text>`              |
//! | `devlog_list`     | `devlogger list [<section>]`                  |
//! | `devlog_sections` | `devlogger sections`                          |
//! | `devlog_update`   | `devlogger update <section> <id> <text>`      |
//! | `devlog_read`     | `devlogger read <section> [<n>]`              |
//! | `devlog_move`     | `devlogger move <from> <id> <to>`             |
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

use indoc::indoc;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo,
    },
    tool, tool_handler, tool_router,
};

use crate::commands::{
    cmd_list, cmd_list_all, cmd_move, cmd_new, cmd_read, cmd_sections, cmd_update,
};
use crate::mcp::args::{ListArgs, MoveArgs, NewArgs, ReadArgs, SectionsArgs, UpdateArgs};
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

    #[tool(name = "devlog_new")]
    #[doc = indoc! {"
        Append a new entry to a section's devlog. Use after a verified fix, feature, or
        non-trivial change — exactly one entry per change, after the work is done.

        Section names must match `[a-z]+(-[a-z]+)*`: lowercase letters and single hyphens
        only. No digits, no underscores. Avoid generic catch-alls like `misc`, `general`, or
        `impl` — they erase the benefit of sectioning. Before creating a new section, call
        `devlog_sections` or `devlog_list` to see what already exists.

        Entry text must be single-line. Keep it terse — you are writing this for your future
        self. Useful signal:
        - What the issue or task was (symptom or root cause).
        - How you handled it (approach, not code).
        - What didn't work first, if anything non-obvious.
        - What resource/doc/file unblocked you, if it was obscure.

        Skip apologies, restating the task, broad project background. The server stamps the
        number and date itself, so don't include them in the text.

        Returns the canonical entry line plus structured fields (number, date, text).
    "}]
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

    #[tool(name = "devlog_list")]
    #[doc = indoc! {"
        List entries — the workhorse for both browsing and for looking up an entry's number
        to pass to `devlog_update`. Call this at the start of a task to skim prior context
        before deciding whether to dive into a specific section with `devlog_read`.

        With a section name, returns only that section's entries. Without one, returns every
        section's entries grouped by section (alphabetical), each line prefixed with
        `[<section>] ` in the text summary.

        The text result is a human-readable one-line-per-entry summary; entries longer than
        ~80 columns are elided with ` (...N more)` — use `devlog_read` if you need the full
        text.
    "}]
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

    #[tool(name = "devlog_sections")]
    #[doc = indoc! {"
        List all section names in alphabetical order. Empty array if no sections exist yet.

        Useful before creating a new section with `devlog_new` — skim what already exists so
        you can reuse an existing section rather than opening a redundant one. If you only
        need the names (not the entries in each section), this is cheaper than `devlog_list`.
        Do NOT create an empty section just to have one.
    "}]
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

    #[tool(name = "devlog_update")]
    #[doc = indoc! {"
        Rewrite an existing entry's text in place. The entry's number and date are preserved;
        only the text changes.

        `id` is either the entry number (from `devlog_list`), an exact `YYYY-MM-DD HH:MM:SS`
        timestamp, or a unique date prefix (e.g. `2026-04-14`).

        Use it to:
        - Collapse a pre-task plan once the work lands — rewrite the planning entry to a
          short pointer to the completion entry (e.g. `Successfully completed: see entry 7`
          or `Failed: see entry 7`). Keeps the log dense and skimmable.
        - Correct an entry that turned out to be wrong or misleading (e.g. the fix you logged
          didn't actually hold, or the root cause was something else).

        Don't use for trivial wording tweaks.
    "}]
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

    #[tool(name = "devlog_read")]
    #[doc = indoc! {"
        Read a section's devlog. With no `n`, returns the entire file verbatim (including any
        non-entry prose). With `n`, returns just the last `n` entry lines.

        Prefer `devlog_list` for skimming — reach for `devlog_read` when you need entries
        verbatim (e.g. a row was elided with ` (...N more)` in `devlog_list`, or you want the
        full section text for context).
    "}]
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

    #[tool(name = "devlog_move")]
    #[doc = indoc! {"
        Move an entry from one section to another. The entry's date is preserved; it slots
        into the destination at its correct chronological position and both sections are
        renumbered 1..N so number order matches date order.

        Returns the moved entry's new canonical line (new number in the destination) plus
        structured fields.
    "}]
    pub async fn devlog_move(
        &self,
        Parameters(args): Parameters<MoveArgs>,
    ) -> Result<CallToolResult, McpError> {
        let base = self.resolve_base(args.base_dir.as_deref());
        let from = args.from_section;
        let id = args.id;
        let to = args.to_section;
        let result = tokio::task::spawn_blocking(move || cmd_move(&base, &from, &id, &to))
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
            indoc! {"
                Append-only markdown devlog. Every entry belongs to a section — there is no
                implicit default log, so every `devlog_new`, `devlog_update`, and `devlog_read`
                requires a section name. Sections live at
                `<base>/DEVLOG/<section>/<section>-devlog.md`.

                When to use proactively:
                - Before a hard task: pre-log a detailed plan — what you're going to do and
                  how. Once the work lands, collapse that entry to a short pointer to the
                  completion entry via `devlog_update`.
                - After a verified fix, feature, or non-trivial debug session: log what
                  happened and how you handled it.
                - Discovering something new and very important.
                - Before starting any task: skim prior context with `devlog_list` (all
                  sections at once), or `devlog_sections` if you just need the section names.
                  Dive into a specific section with `devlog_read` only if you need the full
                  text.
                - Other journal-worthy moments — the devlog is your own working memory across
                  sessions, not only a changelog.

                When NOT to use:
                - Trivial one-line changes that add no future value to re-read.
                - Status updates or messages aimed at the human — this is a journal for
                  yourself. Nobody else reads it.

                All tools accept an optional `base_dir` to target a different project root.
            "}
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
