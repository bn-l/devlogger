//! Tests for server registration surface: `ServerInfo`, capabilities,
//! and tool listing.

use rmcp::ServerHandler;

use super::common::*;

#[tokio::test]
async fn server_info_advertises_tools_capability() {
    let (server, _dir) = fresh_server();
    let info = server.get_info();
    assert!(
        info.capabilities.tools.is_some(),
        "server must advertise the `tools` capability"
    );
}

#[tokio::test]
async fn server_info_reports_crate_name_and_version() {
    let (server, _dir) = fresh_server();
    let info = server.get_info();
    assert_eq!(info.server_info.name, "devlogger");
    assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn server_info_has_instructions_mentioning_each_tool() {
    let (server, _dir) = fresh_server();
    let info = server.get_info();
    let instructions = info.instructions.expect("expected instructions string");
    for name in [
        "devlog_new",
        "devlog_list",
        "devlog_sections",
        "devlog_update",
        "devlog_read",
    ] {
        assert!(
            instructions.contains(name),
            "instructions should mention {name}"
        );
    }
}

#[tokio::test]
async fn default_base_is_exposed_on_the_server() {
    let (server, dir) = fresh_server();
    assert_eq!(server.default_base(), dir.path());
}
