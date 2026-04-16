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

use clap::Parser;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;

use devlogger::mcp::DevlogServer;

#[derive(Debug, Parser)]
#[command(
    name = "devlogger-mcp",
    version,
    about = "stdio MCP server for the devlogger devlog CLI",
    long_about = "Exposes the devlogger commands (new, list, sections, update, read) as MCP tools over a \
                  stdio transport. Typically spawned by an MCP-capable host (Claude Desktop, Claude Code, \
                  Cursor). The host sends JSON-RPC 2.0 on stdin and receives responses on stdout; stderr \
                  is free for diagnostic logging (set RUST_LOG=devlogger_mcp=debug for verbose output)."
)]
struct Args {
    /// Default directory that contains (or will contain) `DEVLOG/`.
    /// Individual tool calls may override this via their `base_dir`
    /// parameter.  Defaults to the current working directory.
    #[arg(short = 'd', long = "dir")]
    dir: Option<PathBuf>,
}

fn main() -> ExitCode {
    // Logging goes to stderr — stdout is the JSON-RPC channel.  Using an
    // env filter lets users crank up verbosity via RUST_LOG when a host
    // integration misbehaves, without making the default output noisy.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn,devlogger=info,devlogger_mcp=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

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

    tracing::info!(base = %base.display(), "devlogger-mcp starting");

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
