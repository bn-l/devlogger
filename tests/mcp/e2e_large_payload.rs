//! Payload-size and encoding stress tests.  These catch three distinct
//! classes of bug that only surface over the wire:
//!
//! 1. Reader buffering — if the server reads stdin with a fixed-size
//!    buffer, a large single-line JSON-RPC frame truncates and the
//!    request fails to parse.
//! 2. Writer fragmentation — if the server writes responses in chunks
//!    without a final newline, the client's line reader stalls.
//! 3. UTF-8 boundary handling — naïve byte slicing on the transport
//!    can split a multi-byte codepoint and corrupt the payload.
//!
//! These all pass trivially for in-process direct calls, so the suite
//! exercises the real `TokioChildProcess` transport.

use serde_json::json;

use super::e2e_common::*;

#[tokio::test]
async fn many_entries_round_trip_over_wire() {
    // 250 entries is well past any reasonable default buffer size and
    // stresses per-call overhead too.  If the server leaks fds, threads,
    // or serializes calls badly, this slows down or times out.
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    const N: u32 = 250;
    for i in 1..=N {
        let r = call_new(&client, "bulk", &format!("entry {i}")).await;
        assert_wire_ok(&r);
        assert_eq!(
            structured(&r).get("number").and_then(|v| v.as_u64()),
            Some(i as u64)
        );
    }

    let listed = call_list(&client, Some("bulk")).await;
    assert_wire_ok(&listed);
    let arr = structured(&listed).as_array().unwrap();
    assert_eq!(arr.len() as u32, N);
    assert_eq!(
        arr.first().unwrap().get("number").and_then(|v| v.as_u64()),
        Some(1)
    );
    assert_eq!(
        arr.last().unwrap().get("number").and_then(|v| v.as_u64()),
        Some(N as u64)
    );

    // `devlog_read` with n covers the tail-only code path for a big file.
    let tail = call_read(&client, "bulk", Some(N as usize)).await;
    assert_wire_ok(&tail);
    let tail_text = text_content(&tail);
    let tail_lines: Vec<&str> = tail_text.lines().collect();
    assert_eq!(tail_lines.len() as u32, N);

    client.cancel().await.ok();
}

#[tokio::test]
async fn single_max_length_entry_text_round_trips_intact() {
    // Entries are capped at `MAX_ENTRY_COLS` columns.  This exercises
    // JSON-RPC framing with the largest single entry the server will
    // accept — enough to tickle any buffering bug on either side.  For
    // aggregate large-frame stress beyond one entry, see
    // `many_entries_round_trip_over_wire`.
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    let big: String = "a".repeat(devlogger::entry::MAX_ENTRY_COLS);
    let created = call(
        &client,
        "devlog_new",
        json!({ "section": "big", "text": big }),
    )
    .await;
    assert_wire_ok(&created);
    assert_eq!(
        structured(&created).get("number").and_then(|v| v.as_u64()),
        Some(1)
    );

    // Read-back via devlog_read must yield the exact text inside the entry line.
    let read = call_read(&client, "big", None).await;
    assert_wire_ok(&read);
    let contents = text_content(&read);
    assert!(
        contents.contains(&big),
        "round-tripped contents missing the original {}-byte payload",
        big.len()
    );

    client.cancel().await.ok();
}

#[tokio::test]
async fn multibyte_unicode_text_round_trips_intact() {
    // Mix of 2-, 3-, and 4-byte codepoints — naïve boundary slicing at
    // the transport layer would corrupt this.  No newlines / control
    // chars, so it passes `validate_entry_text`.
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    let text = "résumé café — 日本語テスト — 🦀🚀✨ — Ω≈ç√∫˜µ≤≥÷";
    let created = call_new(&client, "uni", text).await;
    assert_wire_ok(&created);
    assert_eq!(
        structured(&created).get("number").and_then(|v| v.as_u64()),
        Some(1)
    );

    let listed = call_list(&client, Some("uni")).await;
    assert_wire_ok(&listed);
    let arr = structured(&listed).as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(
        arr[0].get("text").and_then(|v| v.as_str()),
        Some(text),
        "unicode payload mangled over the wire"
    );

    client.cancel().await.ok();
}

#[tokio::test]
async fn long_section_name_writes_and_reads_back() {
    // Section names are [a-z]+(-[a-z]+)*; make one that's long enough to
    // stress path-joining on disk and JSON encoding on the wire.
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    let section: String = (0..20)
        .map(|_| "abcdefghij")
        .collect::<Vec<_>>()
        .join("-");
    assert_eq!(section.len(), 20 * 10 + 19);

    let r = call_new(&client, &section, "ok").await;
    assert_wire_ok(&r);

    let sections = call_sections(&client).await;
    assert_wire_ok(&sections);
    let names: Vec<&str> = structured(&sections)
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(names, vec![section.as_str()]);

    client.cancel().await.ok();
}
