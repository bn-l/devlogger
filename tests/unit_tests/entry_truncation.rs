use chrono::{Local, TimeZone};
use devlogger::entry::{Entry, truncate_line};
use unicode_width::UnicodeWidthStr;

fn cols(s: &str) -> usize {
    s.width()
}

#[test]
fn short_line_passes_through_unchanged() {
    let s = "- 1 | 2026-04-14 11:02:37: hello";
    assert_eq!(truncate_line(s, 80), s);
}

#[test]
fn line_exactly_at_max_passes_through_unchanged() {
    // 80 ASCII chars == 80 cols.
    let s: String = "x".repeat(80);
    assert_eq!(truncate_line(&s, 80), s);
}

#[test]
fn line_one_over_max_is_truncated_with_suffix() {
    let s: String = "x".repeat(81);
    let out = truncate_line(&s, 80);
    assert!(out.ends_with(" more)"), "got: {out}");
    assert!(out.contains("(..."), "got: {out}");
    assert!(cols(&out) <= 80, "width={}: {out}", cols(&out));
}

#[test]
fn very_long_line_reports_correct_elided_count_in_chars() {
    let s: String = "a".repeat(200);
    let out = truncate_line(&s, 80);
    assert!(cols(&out) <= 80);

    let open = out.rfind("(...").expect("suffix present");
    let close = out.rfind(" more)").expect("suffix present");
    let reported: usize = out[open + 4..close].parse().expect("integer N");

    let suffix_chars = out[open..].chars().count() + 1; // +1 for leading space
    let kept_chars = out.chars().count() - suffix_chars;
    assert_eq!(kept_chars + reported, 200, "reported={reported}: {out}");
}

#[test]
fn cjk_line_truncated_to_at_most_80_display_cols() {
    // 100 CJK chars = 200 display columns if untouched.
    let s: String = "漢".repeat(100);
    let out = truncate_line(&s, 80);
    assert!(cols(&out) <= 80, "width={}: {out}", cols(&out));
    assert!(out.contains("(..."));
    // Never split a char mid-byte.
    assert!(out.is_char_boundary(out.len()));
}

#[test]
fn emoji_line_truncated_cleanly_to_display_width() {
    // Most emoji render at width 2.
    let s: String = "🚀".repeat(100);
    let out = truncate_line(&s, 80);
    assert!(cols(&out) <= 80, "width={}: {out}", cols(&out));
    assert!(out.starts_with("🚀"), "got: {out}");
}

#[test]
fn entry_to_line_truncated_wraps_to_line() {
    let date = Local.with_ymd_and_hms(2026, 4, 14, 11, 2, 37).unwrap();
    let long_text = "x".repeat(200);
    let e = Entry::new(1, date, long_text);
    let out = e.to_line_truncated(80);
    assert!(cols(&out) <= 80);
    assert!(out.starts_with("- 1 | 2026-04-14 11:02:37: "));
    assert!(out.ends_with(" more)"));
}

#[test]
fn entry_short_text_is_not_truncated() {
    let date = Local.with_ymd_and_hms(2026, 4, 14, 11, 2, 37).unwrap();
    let e = Entry::new(1, date, "hello");
    assert_eq!(e.to_line_truncated(80), e.to_line());
}

#[test]
fn digit_count_in_suffix_grows_with_elided_size() {
    // 1100 chars elided ⇒ 4-digit N ⇒ suffix steals one extra col from kept.
    let s: String = "a".repeat(1100);
    let out = truncate_line(&s, 80);
    assert!(cols(&out) <= 80);

    let open = out.rfind("(...").unwrap();
    let close = out.rfind(" more)").unwrap();
    let reported: usize = out[open + 4..close].parse().unwrap();
    let suffix_chars = out[open..].chars().count() + 1;
    let kept_chars = out.chars().count() - suffix_chars;
    assert_eq!(kept_chars + reported, 1100);
}

#[test]
fn mixed_ascii_and_wide_respects_column_budget() {
    // Prefix is ASCII (1 col each), then wide chars (2 cols each).
    let s = format!("{}{}", "x".repeat(10), "漢".repeat(60));
    let out = truncate_line(&s, 80);
    assert!(cols(&out) <= 80, "width={}: {out}", cols(&out));
    assert!(out.starts_with("xxxxxxxxxx"));
}
