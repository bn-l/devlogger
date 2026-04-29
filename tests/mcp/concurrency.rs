//! Concurrency tests for the MCP tool surface.
//!
//! The core library serializes writes per-section via a sidecar flock
//! (`.devlogger.lock`); the MCP server wraps each call in
//! `spawn_blocking`, so the tokio runtime can dispatch many calls in
//! parallel.  These tests hammer the happy path concurrently and check
//! that every write landed with a unique, contiguous entry number.

use std::collections::HashSet;

use super::common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_new_calls_produce_unique_contiguous_numbers() {
    let (server, dir) = fresh_server();
    const N: u32 = 16;

    let mut handles = Vec::new();
    for i in 0..N {
        let server = server.clone();
        let h = tokio::spawn(async move {
            server
                .devlog_new(new_args("core", &format!("entry {i}")))
                .await
                .unwrap()
        });
        handles.push(h);
    }

    let mut numbers: Vec<u64> = Vec::with_capacity(N as usize);
    for h in handles {
        let result = h.await.unwrap();
        assert_ok(&result);
        let n = structured(&result)
            .get("number")
            .and_then(|v| v.as_u64())
            .expect("number field");
        numbers.push(n);
    }

    numbers.sort_unstable();
    let expected: Vec<u64> = (1..=N as u64).collect();
    assert_eq!(numbers, expected, "every call must receive a unique number");

    // Disk must reflect the same count.
    let contents = std::fs::read_to_string(section_file(dir.path(), "core")).unwrap();
    let on_disk = contents.lines().filter(|l| l.starts_with("- ")).count();
    assert_eq!(on_disk as u32, N);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_mixed_sections_do_not_interfere() {
    let (server, dir) = fresh_server();

    let mut handles = Vec::new();
    for (section, count) in [("alpha", 8u32), ("beta", 8u32)] {
        for i in 0..count {
            let server = server.clone();
            let h = tokio::spawn(async move {
                server
                    .devlog_new(new_args(section, &format!("{section}-{i}")))
                    .await
                    .unwrap()
            });
            handles.push(h);
        }
    }
    for h in handles {
        assert_ok(&h.await.unwrap());
    }

    for (section, count) in [("alpha", 8u32), ("beta", 8u32)] {
        let contents = std::fs::read_to_string(section_file(dir.path(), section)).unwrap();
        let nums: HashSet<&str> = contents.lines().filter(|l| l.starts_with("- ")).collect();
        assert_eq!(nums.len() as u32, count, "section {section}");
    }
}
