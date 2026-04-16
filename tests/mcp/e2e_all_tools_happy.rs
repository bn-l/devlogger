//! Happy-path round-trip for every tool, exercised over the real
//! stdio/JSON-RPC wire.  This is what catches regressions in tool
//! naming, argument schemas, and result serialization that an
//! in-process direct call cannot.

use serde_json::json;

use super::e2e_common::*;

#[tokio::test]
async fn devlog_new_over_wire_writes_and_returns_entry() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    let result = call_new(&client, "wire", "first").await;
    assert_wire_ok(&result);

    let s = structured(&result);
    assert_eq!(s.get("number").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(s.get("text").and_then(|v| v.as_str()), Some("first"));

    let line = text_content(&result);
    assert!(line.starts_with("- 1 | "));
    assert!(line.ends_with(": first"));

    assert!(section_file(base.path(), "wire").is_file());
    client.cancel().await.ok();
}

#[tokio::test]
async fn devlog_sections_over_wire_round_trips_alphabetical() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    for s in ["zulu", "alpha", "mid"] {
        assert_wire_ok(&call_new(&client, s, "x").await);
    }

    let result = call_sections(&client).await;
    assert_wire_ok(&result);
    let names: Vec<&str> = structured(&result)
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["alpha", "mid", "zulu"]);
    client.cancel().await.ok();
}

#[tokio::test]
async fn devlog_list_with_section_over_wire_returns_ordered_entries() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    for i in 1..=3 {
        assert_wire_ok(&call_new(&client, "seq", &format!("item {i}")).await);
    }

    let result = call_list(&client, Some("seq")).await;
    assert_wire_ok(&result);
    let arr = structured(&result).as_array().unwrap();
    assert_eq!(arr.len(), 3);
    for (i, v) in arr.iter().enumerate() {
        assert_eq!(v.get("number").and_then(|n| n.as_u64()).unwrap(), (i + 1) as u64);
    }
    client.cancel().await.ok();
}

#[tokio::test]
async fn devlog_list_without_section_over_wire_groups_by_section() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    assert_wire_ok(&call_new(&client, "alpha", "a1").await);
    assert_wire_ok(&call_new(&client, "alpha", "a2").await);
    assert_wire_ok(&call_new(&client, "beta", "b1").await);

    let result = call_list(&client, None).await;
    assert_wire_ok(&result);
    let groups = structured(&result).as_array().unwrap();
    assert_eq!(groups.len(), 2);

    let alpha = &groups[0];
    assert_eq!(alpha.get("section").and_then(|v| v.as_str()), Some("alpha"));
    assert_eq!(alpha.get("entries").unwrap().as_array().unwrap().len(), 2);

    let beta = &groups[1];
    assert_eq!(beta.get("section").and_then(|v| v.as_str()), Some("beta"));
    assert_eq!(beta.get("entries").unwrap().as_array().unwrap().len(), 1);

    client.cancel().await.ok();
}

#[tokio::test]
async fn devlog_read_full_over_wire_returns_exact_file_bytes() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    assert_wire_ok(&call_new(&client, "readme", "line one").await);
    assert_wire_ok(&call_new(&client, "readme", "line two").await);

    let disk = std::fs::read_to_string(section_file(base.path(), "readme")).unwrap();
    let result = call_read(&client, "readme", None).await;
    assert_wire_ok(&result);
    assert_eq!(text_content(&result), disk);
    assert_eq!(
        structured(&result)
            .get("contents")
            .and_then(|v| v.as_str()),
        Some(disk.as_str())
    );
    client.cancel().await.ok();
}

#[tokio::test]
async fn devlog_read_last_n_over_wire_returns_entry_lines_only() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    for i in 1..=5 {
        assert_wire_ok(&call_new(&client, "tail", &format!("{i}")).await);
    }
    // Inject a prose line directly so we can confirm read-n filters it.
    let path = section_file(base.path(), "tail");
    let mut raw = std::fs::read_to_string(&path).unwrap();
    raw.push_str("prose line, not an entry\n");
    std::fs::write(&path, raw).unwrap();

    let result = call_read(&client, "tail", Some(2)).await;
    assert_wire_ok(&result);
    let text = text_content(&result);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("- 4 | "));
    assert!(lines[1].starts_with("- 5 | "));
    client.cancel().await.ok();
}

#[tokio::test]
async fn devlog_update_over_wire_preserves_number_and_date() {
    let base = fresh_base();
    let client = spawn_subprocess_client(base.path()).await;

    // Seed via the wire so update goes through the same path.
    let created = call_new(&client, "edits", "before").await;
    assert_wire_ok(&created);
    let original_date = structured(&created)
        .get("date")
        .and_then(|v| v.as_str())
        .unwrap()
        .to_string();

    let updated = call_update(&client, "edits", "1", "after").await;
    assert_wire_ok(&updated);

    let s = structured(&updated);
    assert_eq!(s.get("number").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(s.get("text").and_then(|v| v.as_str()), Some("after"));
    assert_eq!(s.get("date").and_then(|v| v.as_str()), Some(original_date.as_str()));

    client.cancel().await.ok();
}

#[tokio::test]
async fn devlog_new_via_wire_with_explicit_base_dir_argument() {
    let default_base = fresh_base();
    let override_base = fresh_base();
    let client = spawn_subprocess_client(default_base.path()).await;

    let result = call(
        &client,
        "devlog_new",
        json!({
            "section": "alt",
            "text": "via-override",
            "base_dir": override_base.path().to_string_lossy(),
        }),
    )
    .await;
    assert_wire_ok(&result);
    assert!(
        section_file(override_base.path(), "alt").is_file(),
        "write must land in override dir"
    );
    assert!(
        !section_file(default_base.path(), "alt").exists(),
        "default base must be untouched"
    );
    client.cancel().await.ok();
}
