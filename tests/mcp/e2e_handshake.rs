//! Wire-level initialize handshake.  Boots the real binary, completes
//! the MCP protocol negotiation, and verifies the server's advertised
//! identity and capabilities.

use std::time::Duration;

use super::e2e_common::*;

#[tokio::test]
async fn initialize_completes_and_reports_crate_identity() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    let info = client.peer_info().expect("peer info after initialize");
    assert_eq!(info.server_info.name, "devlogger");
    assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
    assert!(info.capabilities.tools.is_some());

    client.cancel().await.ok();
}

#[tokio::test]
async fn initialize_completes_within_ten_seconds() {
    let base = fresh_base();
    let client = tokio::time::timeout(
        Duration::from_secs(10),
        spawn_subprocess_client(base.path()),
    )
    .await
    .expect("initialize should not time out");
    client.cancel().await.ok();
}

#[tokio::test]
async fn server_instructions_are_non_empty_and_mention_every_tool() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;
    let instructions = client
        .peer_info()
        .unwrap()
        .instructions
        .clone()
        .expect("instructions should be advertised");
    assert!(!instructions.is_empty());
    for tool in [
        "devlog_new",
        "devlog_list",
        "devlog_sections",
        "devlog_update",
        "devlog_read",
    ] {
        assert!(
            instructions.contains(tool),
            "instructions should mention {tool}"
        );
    }
    client.cancel().await.ok();
}

#[tokio::test]
async fn repeated_initialize_cycles_leave_no_leftover_state() {
    // Re-spawning the server many times must succeed.  Regressions here
    // would indicate port/socket/file leaks or a handshake that
    // accumulates state it shouldn't.
    for _ in 0..5 {
        let base = fresh_base();
        let client = spawn_subprocess_client(base.path()).await;
        let info = client.peer_info().unwrap().clone();
        assert_eq!(info.server_info.name, "devlogger");
        client.cancel().await.ok();
    }
}

#[tokio::test]
async fn tools_list_exposes_the_full_devlog_surface() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;
    let tools = client.list_all_tools().await.expect("tools/list");
    let names: std::collections::HashSet<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    for expected in [
        "devlog_new",
        "devlog_list",
        "devlog_sections",
        "devlog_update",
        "devlog_read",
        "devlog_move",
    ] {
        assert!(
            names.contains(&expected),
            "{expected} missing from tools/list; got {names:?}"
        );
    }
    assert_eq!(
        tools.len(),
        6,
        "unexpected extra tools; got {:?}",
        tools.iter().map(|t| &t.name).collect::<Vec<_>>()
    );
    client.cancel().await.ok();
}
