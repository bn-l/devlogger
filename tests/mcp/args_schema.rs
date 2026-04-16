//! Tests that each tool-arg struct produces a JSON Schema with the
//! right fields and required-ness.  These schemas are what MCP clients
//! use to build their tool-call forms — regressions are visible to the
//! LLM, not just the code.

use schemars::schema_for;
use serde_json::Value;

use devlogger::mcp::args::{ListArgs, NewArgs, ReadArgs, SectionsArgs, UpdateArgs};

fn schema_json<T: schemars::JsonSchema>() -> Value {
    serde_json::to_value(schema_for!(T)).unwrap()
}

fn required(schema: &Value) -> Vec<&str> {
    schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default()
}

fn properties(schema: &Value) -> &Value {
    schema.get("properties").expect("schema should have properties")
}

#[test]
fn new_args_schema_has_required_section_and_text() {
    let s = schema_json::<NewArgs>();
    let req = required(&s);
    assert!(req.contains(&"section"), "required={req:?}");
    assert!(req.contains(&"text"), "required={req:?}");
    assert!(!req.contains(&"base_dir"), "base_dir must be optional");

    let props = properties(&s);
    for field in ["section", "text", "base_dir"] {
        assert!(props.get(field).is_some(), "missing property {field}");
    }
}

#[test]
fn list_args_schema_has_no_required_fields() {
    let s = schema_json::<ListArgs>();
    let req = required(&s);
    assert!(req.is_empty(), "ListArgs has no required fields; got {req:?}");
}

#[test]
fn sections_args_schema_has_no_required_fields() {
    let s = schema_json::<SectionsArgs>();
    let req = required(&s);
    assert!(
        req.is_empty(),
        "SectionsArgs has no required fields; got {req:?}"
    );
}

#[test]
fn update_args_schema_requires_section_id_and_text() {
    let s = schema_json::<UpdateArgs>();
    let req = required(&s);
    for field in ["section", "id", "text"] {
        assert!(req.contains(&field), "missing required field {field}: {req:?}");
    }
    assert!(!req.contains(&"base_dir"), "base_dir must be optional");
}

#[test]
fn read_args_schema_requires_only_section() {
    let s = schema_json::<ReadArgs>();
    let req = required(&s);
    assert_eq!(req, vec!["section"], "got {req:?}");

    let props = properties(&s);
    for field in ["section", "n", "base_dir"] {
        assert!(props.get(field).is_some(), "missing property {field}");
    }
}

#[test]
fn arg_descriptions_travel_from_doc_comments_to_schema() {
    // schemars 1.x captures doc comments as `description` on each
    // property.  We don't lock in exact text — just that it's non-empty,
    // since that's what an LLM sees.
    let s = schema_json::<NewArgs>();
    let props = properties(&s);
    for field in ["section", "text", "base_dir"] {
        let desc = props[field].get("description").and_then(|v| v.as_str());
        assert!(
            desc.is_some_and(|d| !d.is_empty()),
            "NewArgs.{field} should have a non-empty description in the schema"
        );
    }
}
