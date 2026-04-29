test *filter:
    cargo test {{ if filter == "" { "" } else { filter } }} -- --test-threads=1

mcp-stall-repro-suite:
    cargo build --bin devlogger-mcp
    cargo build --offline --manifest-path research/original-stall-repro/rmcp-blocking-pool/Cargo.toml
    node research/original-stall-repro/run-suite.mjs
