use std::path::Path;

use chrono::{Local, TimeZone};
use devlogger::entry::{Entry, parse_file};

#[test]
fn to_line_matches_canonical_format() {
    let date = Local.with_ymd_and_hms(2026, 4, 14, 11, 2, 37).unwrap();
    let e = Entry::new(1, date, "hello");
    assert_eq!(e.to_line(), "- 1 | 2026-04-14 11:02:37: hello");
}

#[test]
fn to_line_preserves_leading_zeroes_in_date() {
    let date = Local.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap();
    let e = Entry::new(42, date, "x");
    assert_eq!(e.to_line(), "- 42 | 2026-01-02 03:04:05: x");
}

#[test]
fn to_line_preserves_large_numbers() {
    let date = Local.with_ymd_and_hms(2026, 4, 14, 0, 0, 0).unwrap();
    let e = Entry::new(9_999, date, "many entries");
    assert_eq!(e.to_line(), "- 9999 | 2026-04-14 00:00:00: many entries");
}

#[test]
fn to_line_preserves_entry_text_verbatim() {
    let date = Local.with_ymd_and_hms(2026, 4, 14, 11, 0, 0).unwrap();
    let e = Entry::new(1, date, "text: with colons | and pipes");
    assert_eq!(
        e.to_line(),
        "- 1 | 2026-04-14 11:00:00: text: with colons | and pipes"
    );
}

#[test]
fn round_trip_entry_to_line_and_back() {
    let date = Local.with_ymd_and_hms(2026, 4, 14, 11, 2, 37).unwrap();
    let original = Entry::new(7, date, "round trip me");
    let line = original.to_line();
    let contents = format!("{line}\n");

    let parsed = parse_file(Path::new("/tmp/x.md"), &contents).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0], original);
}

#[test]
fn round_trip_with_pipes_and_colons_in_text() {
    let date = Local.with_ymd_and_hms(2026, 4, 14, 11, 0, 0).unwrap();
    let original = Entry::new(3, date, "deploy: staging | then prod");
    let line = original.to_line();
    let contents = format!("{line}\n");

    let parsed = parse_file(Path::new("/tmp/x.md"), &contents).unwrap();
    assert_eq!(parsed[0], original);
}

#[test]
fn round_trip_with_unicode_text() {
    let date = Local.with_ymd_and_hms(2026, 4, 14, 11, 0, 0).unwrap();
    let original = Entry::new(1, date, "café 漢字 🚀 — em-dash");
    let contents = format!("{}\n", original.to_line());
    let parsed = parse_file(Path::new("/tmp/x.md"), &contents).unwrap();
    assert_eq!(parsed[0], original);
}

#[test]
fn round_trip_with_empty_text() {
    let date = Local.with_ymd_and_hms(2026, 4, 14, 11, 0, 0).unwrap();
    let original = Entry::new(1, date, "");
    let contents = format!("{}\n", original.to_line());
    let parsed = parse_file(Path::new("/tmp/x.md"), &contents).unwrap();
    assert_eq!(parsed[0], original);
}

#[test]
fn round_trip_with_tab_in_text() {
    let date = Local.with_ymd_and_hms(2026, 4, 14, 11, 0, 0).unwrap();
    let original = Entry::new(1, date, "col1\tcol2\tcol3");
    let contents = format!("{}\n", original.to_line());
    let parsed = parse_file(Path::new("/tmp/x.md"), &contents).unwrap();
    assert_eq!(parsed[0], original);
}

#[test]
fn round_trip_with_max_u32_number() {
    let date = Local.with_ymd_and_hms(2026, 4, 14, 11, 0, 0).unwrap();
    let original = Entry::new(u32::MAX, date, "last entry possible");
    let contents = format!("{}\n", original.to_line());
    let parsed = parse_file(Path::new("/tmp/x.md"), &contents).unwrap();
    assert_eq!(parsed[0], original);
}
