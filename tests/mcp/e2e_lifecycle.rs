//! Lifecycle tests: cancellation, clean shutdown, multi-session
//! isolation.  Regression guards for bugs like "server hangs on the
//! `notifications/initialized` message" that have shown up in other
//! stdio MCP servers.

use std::time::Duration;

use super::e2e_common::*;

#[tokio::test]
async fn client_cancel_terminates_the_server_cleanly() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    // Do some work to prove the session is live.
    assert_wire_ok(&call_new(&client, "core", "x").await);

    // Cancel must resolve, not hang.  Give it a generous budget since
    // subprocess teardown involves a process wait.
    tokio::time::timeout(Duration::from_secs(10), client.cancel())
        .await
        .expect("cancel should not hang")
        .ok();
}

#[tokio::test]
async fn two_independent_server_processes_do_not_share_state() {
    let base_a = fresh_base();
    let base_b = fresh_base();

    let client_a = spawn_subprocess_client(base_a.path()).await;
    let client_b = spawn_subprocess_client(base_b.path()).await;

    // Write different data to each server.
    assert_wire_ok(&call_new(&client_a, "left", "from-a").await);
    assert_wire_ok(&call_new(&client_b, "right", "from-b").await);

    // Each sees only its own section.
    let sections_a: Vec<String> = structured(&call_sections(&client_a).await)
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let sections_b: Vec<String> = structured(&call_sections(&client_b).await)
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(sections_a, vec!["left"]);
    assert_eq!(sections_b, vec!["right"]);

    client_a.cancel().await.ok();
    client_b.cancel().await.ok();
}

#[tokio::test]
async fn session_survives_sustained_tool_call_volume() {
    // Rules out subtle memory/fd leaks per-call: 50 round trips in a
    // row.  If the server leaks file descriptors or accumulates state,
    // one of these will time out or fail.
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    for i in 0..50 {
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            call_new(&client, "load", &format!("entry {i}")),
        )
        .await
        .expect("tool call should complete within 5s");
        assert_wire_ok(&result);
    }

    let listed = call_list(&client, Some("load")).await;
    assert_eq!(structured(&listed).as_array().unwrap().len(), 50);

    client.cancel().await.ok();
}

#[tokio::test]
async fn initialize_and_list_tools_is_fast() {
    // A sanity bound that also catches the "server spins in a busy loop
    // after init" bug class.
    let base = fresh_base();
    let started = std::time::Instant::now();
    let client = spawn_subprocess_client(base.path()).await;
    let _tools = client.list_all_tools().await.unwrap();
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(10),
        "init+tools/list took {elapsed:?}"
    );
    client.cancel().await.ok();
}
