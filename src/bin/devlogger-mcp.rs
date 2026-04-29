//! `devlogger-mcp` — stdio MCP server for the devlogger CLI.
//!
//! Spawned by an MCP-capable host (Claude Desktop, Claude Code, Cursor,
//! etc.) as a child process.  Communicates over stdio using MCP's
//! JSON-RPC 2.0 wire format — which means **no prints to stdout** from
//! anywhere in the process except the rmcp transport itself.  All
//! logging is unconditionally routed to stderr.
//!
//! Configure the default base directory (the parent of `DEVLOG/`) with
//! `--dir` / `-d`; individual tool calls may still override via their
//! `base_dir` argument.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Mutex;

use clap::Parser;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::{Layer as _, SubscriberExt};

use devlogger::mcp::DevlogServer;

const DEFAULT_STDERR_FILTER: &str = "warn,devlogger_mcp=info";
const FILE_LOG_FILTER: &str = "warn,devlogger::mcp::server=info,devlogger_mcp=info";

#[derive(Debug, Parser)]
#[command(
    name = "devlogger-mcp",
    version,
    about = "stdio MCP server for the devlogger devlog CLI",
    long_about = "Exposes the devlogger commands (new, list, sections, update, read) as MCP tools over a \
                  stdio transport. Typically spawned by an MCP-capable host (Claude Desktop, Claude Code, \
                  Cursor). The host sends JSON-RPC 2.0 on stdin and receives responses on stdout; stderr \
                  is free for diagnostic logging (set RUST_LOG=devlogger_mcp=debug for verbose output).\n\n\
                  Server logs are written as JSONL to ~/.local/share/devlogger/logs/mcp-server.<date>.\n\
                  Override the log directory with DEVLOGGER_LOG_DIR."
)]
struct Args {
    /// Default directory that contains (or will contain) `DEVLOG/`.
    /// Individual tool calls may override this via their `base_dir`
    /// parameter.  Defaults to the current working directory.
    #[arg(short = 'd', long = "dir")]
    dir: Option<PathBuf>,
}

/// Build the log directory path.  `DEVLOGGER_LOG_DIR` takes priority,
/// then `$HOME/.local/share/devlogger/logs`.  Returns `None` if neither
/// is set.
fn log_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("DEVLOGGER_LOG_DIR") {
        return Some(PathBuf::from(d));
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share/devlogger/logs"))
}

/// Try to open (or create) today's log file in append mode.  Returns
/// `None` on any failure — the server must never crash because of
/// logging.
fn open_log_file() -> Option<std::fs::File> {
    let dir = log_dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    let today = chrono::Local::now().format("%Y-%m-%d");
    let path = dir.join(format!("mcp-server.{today}.jsonl"));
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok()
}

fn stderr_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_STDERR_FILTER))
}

fn file_log_filter() -> EnvFilter {
    EnvFilter::new(FILE_LOG_FILTER)
}

fn main() -> ExitCode {
    // Stderr layer — human-readable, always active.
    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_filter(stderr_filter());

    // File layer — structured JSONL, best-effort.  Uses synchronous
    // Mutex<File> writes — fine for this low-volume server and
    // guarantees every log line is flushed to disk immediately.
    let file_layer = open_log_file().map(|file| {
        tracing_subscriber::fmt::layer()
            .json()
            .with_writer(Mutex::new(file))
            .with_filter(file_log_filter())
    });

    let subscriber = tracing_subscriber::registry()
        .with(stderr_layer)
        .with(file_layer);
    tracing::subscriber::set_global_default(subscriber).expect("failed to set tracing subscriber");

    let args = Args::parse();

    let base = match args.dir {
        Some(p) => p,
        None => match std::env::current_dir() {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("cannot determine current working directory: {e}");
                return ExitCode::FAILURE;
            }
        },
    };

    // Gated at debug: stdio MCP hosts surface stderr as a connection
    // log line (Claude Code writes `{"error":"Server stderr: …"}` per
    // byte).  A startup banner on every reconnect is pure noise for
    // the user and burns a slot in the host's log buffer.  Users who
    // want the banner can set `RUST_LOG=devlogger_mcp=debug`.
    tracing::debug!(base = %base.display(), "devlogger-mcp starting");

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("failed to build tokio runtime: {e}");
            return ExitCode::FAILURE;
        }
    };

    match runtime.block_on(run(base)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("devlogger-mcp exited with error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run(base: PathBuf) -> eyre::Result<()> {
    let server = DevlogServer::new(base);
    let service = server
        .serve(stdio())
        .await
        .map_err(|e| eyre::eyre!("failed to start MCP service: {e}"))?;
    service
        .waiting()
        .await
        .map_err(|e| eyre::eyre!("MCP service terminated abnormally: {e}"))?;
    Ok(())
}
