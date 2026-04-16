//! Protocol-layer concurrency.  Many in-flight tool calls share a
//! single stdio pipe; rmcp multiplexes them by request id.  The server
//! must not serialize responses in call order — we don't assert order,
//! but we do assert uniqueness and contiguous numbering (thanks to the
//! library's flock).

use std::collections::HashSet;

use super::e2e_common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn many_concurrent_new_calls_share_one_stdio_pipe() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;
    let client = std::sync::Arc::new(client);
    const N: u32 = 24;

    let mut handles = Vec::new();
    for i in 0..N {
        let c = client.clone();
        handles.push(tokio::spawn(async move {
            call_new(&c, "wire", &format!("entry {i}")).await
        }));
    }

    let mut numbers: Vec<u64> = Vec::with_capacity(N as usize);
    for h in handles {
        let result = h.await.unwrap();
        assert_wire_ok(&result);
        numbers.push(
            structured(&result)
                .get("number")
                .and_then(|v| v.as_u64())
                .unwrap(),
        );
    }
    numbers.sort_unstable();
    assert_eq!(numbers, (1..=N as u64).collect::<Vec<_>>());

    // On-disk count matches.
    let file = std::fs::read_to_string(section_file(base.path(), "wire")).unwrap();
    assert_eq!(
        file.lines().filter(|l| l.starts_with("- ")).count() as u32,
        N
    );

    std::sync::Arc::into_inner(client)
        .unwrap()
        .cancel()
        .await
        .ok();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_mixed_tool_calls_do_not_cross_contaminate() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;
    let client = std::sync::Arc::new(client);

    // 12 writes across 3 sections, interleaved with 6 list calls.
    let mut handles = Vec::new();
    for (section, tag) in [("alpha", "a"), ("beta", "b"), ("gamma", "g")] {
        for i in 0..4 {
            let c = client.clone();
            handles.push(tokio::spawn(async move {
                call_new(&c, section, &format!("{tag}-{i}")).await
            }));
        }
    }
    for _ in 0..6 {
        let c = client.clone();
        handles.push(tokio::spawn(async move { call_sections(&c).await }));
    }

    for h in handles {
        let r = h.await.unwrap();
        assert!(
            r.is_error != Some(true),
            "one of the concurrent calls errored: {}",
            text_content(&r)
        );
    }

    // After the dust settles, every section should have exactly 4 entries.
    for section in ["alpha", "beta", "gamma"] {
        let contents = std::fs::read_to_string(section_file(base.path(), section)).unwrap();
        let nums: HashSet<&str> = contents
            .lines()
            .filter(|l| l.starts_with("- "))
            .collect();
        assert_eq!(nums.len(), 4, "section {section}");
    }

    std::sync::Arc::into_inner(client)
        .unwrap()
        .cancel()
        .await
        .ok();
}
