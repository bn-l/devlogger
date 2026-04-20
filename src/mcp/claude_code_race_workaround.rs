//! # ⚠️  CLAUDE CODE `initialize` RACE WORKAROUND — DELETE WHEN FIXED  ⚠️
//!
//! Claude Code 2.1.114 (and nearby versions) has a race in its MCP
//! client where very-fast stdio servers lose their first
//! post-initialize `tools/list` response: the server answers with the
//! request's `id`, but Claude Code's request-id table has already
//! freed that slot and logs the response as `Received a response for
//! an unknown message ID`.  A 60-second timer then fires and the
//! transport is torn down, so the user sees the devlogger MCP
//! "disconnect" every minute or so.
//!
//! We caught this by cross-referencing the MCP log files under
//! `~/Library/Caches/claude-cli-nodejs/.../mcp-logs-*`.  In the exact
//! same Claude Code session that killed devlogger, `textbelt` (node
//! stdio, ~1284 ms handshake) and `context7` (HTTP, ~1342 ms) were
//! fine.  Devlogger, a native Rust binary, answered `initialize` in
//! ~3 ms and lost the race every time.  We then proved the server
//! itself is healthy with `tests/mcp/e2e_timing.rs` — `tools/list`
//! round-trips in ~163 µs, and every tool call in sub-millisecond
//! time.  The 60-second delay is 100 % on Claude Code's side.
//!
//! The cursed-but-correct workaround is to delay `initialize` past
//! the race window.  The slowest "always works" data point we have
//! is `textbelt` at ~1.28 s; the fastest confirmed-broken one is
//! devlogger itself at ~3 ms.  We picked 500 ms — well into the
//! range that Claude Code's client seems to tolerate, while still
//! keeping the handshake snappy.  If reconnects resume, bump this
//! up toward textbelt's latency before assuming the workaround is
//! wrong.
//!
//! ## Why this lives in its own file
//!
//! To make it impossible to forget.  `git grep CLAUDE_CODE_RACE` or
//! `WORKAROUND` lands you right here; removing the sleep requires
//! touching a file whose whole purpose is spelled out in the module
//! header.  A time-bomb test in `tests/mcp/e2e_timing.rs` also starts
//! failing on [`REVIEW_BY`], forcing a human review on that date.
//!
//! ## When to delete
//!
//! When Claude Code ships a release that fixes the MCP client race
//! AND that release has been in the wild long enough that you are no
//! longer seeing reconnects.  At that point:
//!
//! 1. Delete this file.
//! 2. Delete the `initialize` override on [`crate::mcp::server::DevlogServer`]
//!    (the default rmcp implementation does the right thing).
//! 3. Remove the `pub mod claude_code_race_workaround;` line from
//!    `src/mcp/mod.rs`.
//! 4. Remove the `workaround_has_not_outlived_its_welcome` and
//!    `initialize_applies_race_workaround_delay` tests in
//!    `tests/mcp/e2e_timing.rs` (or relax the assertion and rename
//!    the test to just check fast initialize).
//!
//! ## Upstream references
//!
//! - <https://github.com/anthropics/claude-code/issues/50095> —
//!   malformed/late MCP response drops the whole stdio transport.
//! - <https://github.com/anthropics/claude-code/issues/43299> —
//!   hardcoded 60 s `requestTimeout` in the embedded
//!   `@modelcontextprotocol/sdk` client.
//! - <https://github.com/anthropics/claude-code/issues/40207> — stdio
//!   MCP servers killed 10–60 s after a successful handshake.
//!
//! ## Empirical evidence for the 500 ms target
//!
//! | server    | observed handshake | race occurs |
//! |-----------|--------------------|-------------|
//! | devlogger |   ~3 ms (native)   |   yes       |
//! | textbelt  |  ~1284 ms (node)   |   no        |
//! | context7  |  ~1342 ms (HTTP)   |   no        |
//!
//! We don't have direct evidence of where the race boundary is
//! between 3 ms and 1284 ms.  500 ms is a compromise: long enough
//! that the race is almost certainly closed (two orders of magnitude
//! above the broken case), short enough that the handshake still
//! feels instant to a user.  If reconnects resume at 500 ms, step up
//! toward textbelt's 1.28 s before assuming the workaround is wrong
//! — do not trim it down without a fresh measurement.

use std::time::Duration;

/// Date after which the time-bomb test in `tests/mcp/e2e_timing.rs`
/// starts failing.  Format: `YYYY-MM-DD`.  When you bump this, also
/// write a line in the DEVLOG explaining what you checked upstream
/// and why the workaround still needs to be there.
pub const REVIEW_BY: &str = "2026-07-20";

/// How long `initialize` stalls before returning its response.
/// 500 ms is two orders of magnitude above devlogger's natural
/// ~3 ms handshake (the broken case) and well below textbelt's
/// ~1.28 s handshake (the known-good case).  See the module header
/// for the full rationale.
pub const INITIALIZE_DELAY: Duration = Duration::from_millis(500);

/// Sleep long enough to slip past Claude Code's ID-tracking race on
/// `initialize`.  Logs a warn on every invocation so anyone tailing
/// the MCP log sees the reason the handshake is artificially slow
/// and can trace it back to this module.
pub async fn stall_initialize() {
    tracing::warn!(
        delay_ms = INITIALIZE_DELAY.as_millis() as u64,
        review_by = REVIEW_BY,
        upstream = "anthropics/claude-code#50095, #43299, #40207",
        "applying claude-code initialize-race workaround — delete src/mcp/claude_code_race_workaround.rs when upstream is fixed"
    );
    tokio::time::sleep(INITIALIZE_DELAY).await;
}
