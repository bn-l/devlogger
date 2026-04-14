use std::path::Path;

use devlogger::entry::{DATE_FORMAT, parse_file};

fn p() -> &'static Path {
    Path::new("/fake/DEVLOG/main-devlog.md")
}

// ---- happy path ----

#[test]
fn parses_single_entry() {
    let contents = "- 1 | 2026-04-14 11:02:37: hello world\n";
    let entries = parse_file(p(), contents).expect("should parse");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].number, 1);
    assert_eq!(entries[0].text, "hello world");
    assert_eq!(entries[0].date.format(DATE_FORMAT).to_string(), "2026-04-14 11:02:37");
}

#[test]
fn parses_multiple_entries_in_order() {
    let contents = "\
- 1 | 2026-04-14 11:02:37: first
- 2 | 2026-04-14 11:03:00: second
- 3 | 2026-04-14 11:04:00: third
";
    let entries = parse_file(p(), contents).unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].number, 1);
    assert_eq!(entries[1].number, 2);
    assert_eq!(entries[2].number, 3);
    assert_eq!(entries[2].text, "third");
}

#[test]
fn ignores_non_entry_lines() {
    // Prose, headings, and blank lines are preserved (ignored by the parser).
    let contents = "\
# My Devlog

Some notes about the project.

- 1 | 2026-04-14 11:02:37: first entry

more prose
- 2 | 2026-04-14 11:03:00: second entry
";
    let entries = parse_file(p(), contents).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].number, 1);
    assert_eq!(entries[1].number, 2);
}

#[test]
fn empty_file_yields_no_entries() {
    let entries = parse_file(p(), "").unwrap();
    assert!(entries.is_empty());
}

#[test]
fn entry_text_may_contain_colons_and_pipes() {
    // split_once splits at the FIRST match, so pipes/colons later in the
    // text are part of `text`, not separators.
    let contents = "- 1 | 2026-04-14 11:02:37: ratio 3:1 | thought about it\n";
    let entries = parse_file(p(), contents).unwrap();
    assert_eq!(entries[0].text, "ratio 3:1 | thought about it");
}

// ---- parse errors: shape ----

#[test]
fn parse_error_missing_pipe() {
    let contents = "- 1 2026-04-14 11:02:37: text\n";
    let err = parse_file(p(), contents).unwrap_err().to_string();
    assert!(err.contains("missing ` | `"), "unexpected: {err}");
    assert!(err.contains(":1:"), "should include line 1: {err}");
}

#[test]
fn parse_error_missing_colon_space() {
    let contents = "- 1 | 2026-04-14 11:02:37 text with no colon-space\n";
    let err = parse_file(p(), contents).unwrap_err().to_string();
    assert!(err.contains("missing `: `"), "unexpected: {err}");
}

#[test]
fn parse_error_bad_number() {
    let contents = "- abc | 2026-04-14 11:02:37: text\n";
    let err = parse_file(p(), contents).unwrap_err().to_string();
    assert!(err.contains("`abc`"), "unexpected: {err}");
    assert!(err.contains("not a positive integer"), "unexpected: {err}");
}

#[test]
fn parse_error_negative_number() {
    let contents = "- -1 | 2026-04-14 11:02:37: text\n";
    let err = parse_file(p(), contents).unwrap_err().to_string();
    assert!(err.contains("not a positive integer"), "unexpected: {err}");
}

#[test]
fn parse_error_bad_date() {
    let contents = "- 1 | not-a-date: text\n";
    let err = parse_file(p(), contents).unwrap_err().to_string();
    assert!(err.contains("not-a-date"), "should quote the bad date: {err}");
    assert!(err.contains("YYYY-MM-DD HH:MM:SS"), "should describe expected format: {err}");
}

#[test]
fn parse_error_partial_date() {
    // Date missing time component.
    let contents = "- 1 | 2026-04-14: text\n";
    let err = parse_file(p(), contents).unwrap_err().to_string();
    assert!(err.contains("YYYY-MM-DD HH:MM:SS"), "unexpected: {err}");
}

// ---- parse errors: location info ----

#[test]
fn parse_error_reports_correct_line_number() {
    // 3 valid lines, then a bad 4th.
    let contents = "\
- 1 | 2026-04-14 11:00:00: ok
- 2 | 2026-04-14 11:00:00: ok
- 3 | 2026-04-14 11:00:00: ok
- broken line here
";
    let err = parse_file(p(), contents).unwrap_err().to_string();
    assert!(err.contains(":4:"), "should point to line 4, got: {err}");
}

#[test]
fn parse_error_counts_non_entry_lines() {
    // Prose on lines 1-3, bad entry on line 4.
    let contents = "\
# Header
some prose
blank follows

- broken
";
    let err = parse_file(p(), contents).unwrap_err().to_string();
    assert!(err.contains(":5:"), "should point to line 5, got: {err}");
}

#[test]
fn parse_error_includes_file_path() {
    let contents = "- broken\n";
    let err = parse_file(Path::new("/some/weird/path.md"), contents)
        .unwrap_err()
        .to_string();
    assert!(err.contains("/some/weird/path.md"), "should include path: {err}");
}

// ---- parse errors: NEVER dump file contents ----

#[test]
fn parse_error_does_not_dump_file_contents() {
    let contents = "\
SECRET_TOKEN=abcdef123
# huge unrelated markdown
line 1
line 2
line 3
line 4
- broken line sentinel
more secret stuff after the error
";
    let err = parse_file(p(), contents).unwrap_err().to_string();
    // The error should NOT contain unrelated file contents.
    assert!(!err.contains("SECRET_TOKEN"), "must not dump file: {err}");
    assert!(!err.contains("huge unrelated markdown"), "must not dump file: {err}");
    assert!(!err.contains("more secret stuff"), "must not dump file: {err}");
    // Error must be short — one line, under a reasonable bound.
    assert!(
        err.lines().count() == 1,
        "error should be one line, got {}: {err}",
        err.lines().count()
    );
    assert!(err.len() < 300, "error should be concise ({} chars): {err}", err.len());
}

#[test]
fn parse_error_does_not_include_the_raw_line() {
    // The raw line content is NOT echoed back — we describe the shape that
    // was expected instead. This avoids dumping potentially sensitive or
    // very long lines into the terminal.
    let contents = "- this_entire_long_line_should_not_be_in_the_error_message_at_all\n";
    let err = parse_file(p(), contents).unwrap_err().to_string();
    assert!(
        !err.contains("this_entire_long_line_should_not_be_in_the_error_message_at_all"),
        "error should not echo the raw line: {err}"
    );
}

// ---- edge cases ----

#[test]
fn parses_file_with_crlf_terminators() {
    // str::lines() splits on both \n and \r\n; the parser must work with
    // either.
    let contents = "- 1 | 2026-04-14 11:02:37: one\r\n- 2 | 2026-04-14 11:03:00: two\r\n";
    let entries = parse_file(p(), contents).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].text, "one");
    assert_eq!(entries[1].text, "two");
}

#[test]
fn parses_file_with_mixed_terminators() {
    let contents = "- 1 | 2026-04-14 11:02:37: one\r\n- 2 | 2026-04-14 11:03:00: two\n";
    let entries = parse_file(p(), contents).unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn parses_entry_with_empty_text() {
    let contents = "- 1 | 2026-04-14 11:02:37: \n";
    let entries = parse_file(p(), contents).unwrap();
    assert_eq!(entries[0].text, "");
}

#[test]
fn parses_entry_with_trailing_whitespace_in_text() {
    let contents = "- 1 | 2026-04-14 11:02:37: text with trailing spaces   \n";
    let entries = parse_file(p(), contents).unwrap();
    assert_eq!(entries[0].text, "text with trailing spaces   ");
}

#[test]
fn parses_large_numbers() {
    let contents = "- 4294967295 | 2026-04-14 11:02:37: max u32\n";
    let entries = parse_file(p(), contents).unwrap();
    assert_eq!(entries[0].number, u32::MAX);
}

#[test]
fn parse_error_on_number_overflow() {
    // u32::MAX + 1
    let contents = "- 4294967296 | 2026-04-14 11:02:37: overflow\n";
    let err = parse_file(p(), contents).unwrap_err().to_string();
    assert!(err.contains("not a positive integer"), "unexpected: {err}");
}

#[test]
fn parse_error_on_decimal_number() {
    let contents = "- 1.5 | 2026-04-14 11:02:37: decimal\n";
    assert!(parse_file(p(), contents).is_err());
}

#[test]
fn line_without_space_after_dash_is_not_an_entry() {
    // `-1 ...` does NOT start with `- ` (two chars), so it's treated as
    // prose and silently ignored, not parsed as an entry.
    let contents = "-1 | 2026-04-14 11:02:37: missing space\n";
    let entries = parse_file(p(), contents).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn plain_dash_line_ignored_as_prose() {
    // A bare "- " with nothing after it is still an entry-prefixed line,
    // so it errors as malformed.  But actually "- " with empty rest.
    let contents = "-\n"; // no space — not an entry prefix
    let entries = parse_file(p(), contents).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn parse_error_on_dash_space_only() {
    // `- ` with nothing after is structurally an entry line but lacks the
    // required fields — parser must complain.
    let contents = "- \n";
    assert!(parse_file(p(), contents).is_err());
}

#[test]
fn parse_file_path_is_preserved_through_error_chain() {
    // Confirm all ParseError variants include the path.
    let cases = [
        "- not_a_number | 2026-04-14 11:00:00: x\n",
        "- 1 | bad-date: x\n",
        "- 1 2026-04-14 11:00:00: x\n", // missing pipe
    ];
    for c in cases {
        let err = parse_file(std::path::Path::new("/abc/XYZ.md"), c)
            .unwrap_err()
            .to_string();
        assert!(err.contains("/abc/XYZ.md"), "case {c:?} → {err}");
    }
}
