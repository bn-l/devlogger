pub mod common;
pub mod e2e_common;

mod args_schema;
mod base_dir;
mod concurrency;
mod convert_roundtrip;
mod errors;
mod registration;
mod result_shape;
mod tool_list;
mod tool_new;
mod tool_read;
mod tool_sections;
mod tool_update;

// Wire-level end-to-end suite.  Each file covers a narrow slice;
// together they exercise every tool, schema, error path, lifecycle
// event, and stdio invariant a real MCP host cares about.
mod e2e_all_tools_errors;
mod e2e_all_tools_happy;
mod e2e_base_dir;
mod e2e_binary_cli;
mod e2e_concurrency;
mod e2e_handshake;
mod e2e_inprocess_duplex;
mod e2e_large_payload;
mod e2e_lifecycle;
mod e2e_schemas;
mod e2e_stdout_cleanliness;
