//! Verify every tool's JSON Schema as exposed by `tools/list` over the
//! wire.  This is what MCP-host tool pickers see — an LLM relies on
//! these to construct well-formed calls.

use std::collections::HashMap;

use rmcp::model::Tool;
use serde_json::Value;

use super::e2e_common::*;

async fn listed_by_name(base: &std::path::Path) -> HashMap<String, Tool> {
    let client = spawn_subprocess_client(base).await;
    let tools = client.list_all_tools().await.expect("tools/list");
    let map = tools
        .into_iter()
        .map(|t| (t.name.to_string(), t))
        .collect();
    client.cancel().await.ok();
    map
}

fn schema(tool: &Tool) -> Value {
    serde_json::to_value(&tool.input_schema).unwrap()
}

fn required(schema: &Value) -> Vec<String> {
    schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn properties(schema: &Value) -> &Value {
    schema.get("properties").expect("missing `properties`")
}

#[tokio::test]
async fn every_tool_has_non_empty_description() {
    let base = fresh_base();
    let tools = listed_by_name(base.path()).await;
    for name in [
        "devlog_new",
        "devlog_list",
        "devlog_sections",
        "devlog_update",
        "devlog_read",
    ] {
        let t = &tools[name];
        let desc = t.description.clone().unwrap_or_default();
        assert!(
            !desc.is_empty(),
            "{name} has an empty description; LLMs need the description to pick tools"
        );
    }
}

#[tokio::test]
async fn devlog_new_schema_required_section_and_text() {
    let base = fresh_base();
    let tools = listed_by_name(base.path()).await;
    let s = schema(&tools["devlog_new"]);
    let req = required(&s);
    assert!(req.contains(&"section".to_string()), "required={req:?}");
    assert!(req.contains(&"text".to_string()), "required={req:?}");
    assert!(!req.contains(&"base_dir".to_string()));

    let props = properties(&s);
    for field in ["section", "text", "base_dir"] {
        assert!(props.get(field).is_some(), "missing {field}");
        let desc = props[field]
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(!desc.is_empty(), "{field} schema has empty description");
    }
}

#[tokio::test]
async fn devlog_list_schema_has_no_required_fields() {
    let base = fresh_base();
    let tools = listed_by_name(base.path()).await;
    let s = schema(&tools["devlog_list"]);
    assert!(required(&s).is_empty(), "got {:?}", required(&s));
    let props = properties(&s);
    for field in ["section", "base_dir"] {
        assert!(props.get(field).is_some(), "missing {field}");
    }
}

#[tokio::test]
async fn devlog_sections_schema_has_only_optional_base_dir() {
    let base = fresh_base();
    let tools = listed_by_name(base.path()).await;
    let s = schema(&tools["devlog_sections"]);
    assert!(required(&s).is_empty());
    assert!(properties(&s).get("base_dir").is_some());
}

#[tokio::test]
async fn devlog_update_schema_requires_section_id_text() {
    let base = fresh_base();
    let tools = listed_by_name(base.path()).await;
    let s = schema(&tools["devlog_update"]);
    let req = required(&s);
    for field in ["section", "id", "text"] {
        assert!(req.contains(&field.to_string()), "missing {field}: {req:?}");
    }
    assert!(!req.contains(&"base_dir".to_string()));
}

#[tokio::test]
async fn devlog_read_schema_requires_only_section() {
    let base = fresh_base();
    let tools = listed_by_name(base.path()).await;
    let s = schema(&tools["devlog_read"]);
    assert_eq!(required(&s), vec!["section".to_string()]);
    let props = properties(&s);
    for field in ["section", "n", "base_dir"] {
        assert!(props.get(field).is_some(), "missing {field}");
    }
}

#[tokio::test]
async fn every_schema_declares_type_object() {
    let base = fresh_base();
    let tools = listed_by_name(base.path()).await;
    for (name, tool) in &tools {
        let s = schema(tool);
        let ty = s.get("type").and_then(|v| v.as_str());
        assert_eq!(
            ty,
            Some("object"),
            "{name} schema should declare type=object; got {ty:?}"
        );
    }
}
