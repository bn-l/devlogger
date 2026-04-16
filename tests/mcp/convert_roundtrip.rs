//! Unit tests for the `mcp::convert` JSON projections.

use chrono::{Local, TimeZone};

use devlogger::entry::Entry;
use devlogger::mcp::convert::{EntryJson, SectionEntriesJson, entries_to_json};

fn fixed_entry(number: u32, text: &str) -> Entry {
    // Use a fixed local datetime that round-trips losslessly through the
    // on-disk format (DST ambiguous/nonexistent dates excluded).
    let date = Local.with_ymd_and_hms(2026, 4, 14, 12, 34, 56).unwrap();
    Entry::new(number, date, text)
}

#[test]
fn entry_json_has_number_date_text_and_line() {
    let e = fixed_entry(42, "example");
    let j = EntryJson::from(&e);
    assert_eq!(j.number, 42);
    assert_eq!(j.text, "example");
    assert_eq!(j.date, "2026-04-14 12:34:56");
    assert_eq!(j.line, "- 42 | 2026-04-14 12:34:56: example");
}

#[test]
fn entry_json_serializes_to_expected_json_shape() {
    let e = fixed_entry(1, "t");
    let v = serde_json::to_value(EntryJson::from(&e)).unwrap();

    assert_eq!(v["number"], 1);
    assert_eq!(v["text"], "t");
    assert_eq!(v["date"], "2026-04-14 12:34:56");
    assert_eq!(v["line"], "- 1 | 2026-04-14 12:34:56: t");
}

#[test]
fn entries_to_json_preserves_input_order() {
    let es = vec![
        fixed_entry(3, "c"),
        fixed_entry(1, "a"),
        fixed_entry(2, "b"),
    ];
    let j = entries_to_json(&es);
    let texts: Vec<&str> = j.iter().map(|e| e.text.as_str()).collect();
    assert_eq!(texts, vec!["c", "a", "b"]);
}

#[test]
fn section_entries_json_serializes_as_object_with_section_and_entries() {
    let payload = SectionEntriesJson {
        section: "core".into(),
        entries: vec![EntryJson::from(&fixed_entry(1, "hi"))],
    };
    let v = serde_json::to_value(&payload).unwrap();
    assert_eq!(v["section"], "core");
    assert!(v["entries"].is_array());
    assert_eq!(v["entries"].as_array().unwrap().len(), 1);
}

#[test]
fn owned_entry_into_entry_json_works() {
    let e = fixed_entry(7, "text");
    let j: EntryJson = e.into();
    assert_eq!(j.number, 7);
    assert_eq!(j.text, "text");
}
