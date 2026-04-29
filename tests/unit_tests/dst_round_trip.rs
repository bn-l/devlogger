//! Regression test: `parse_file` must not reject timestamps that fall
//! inside a DST fall-back repeated hour.  The on-disk format has no
//! offset, so the same naive local time can map to two different UTC
//! instants during that window.  We deterministically pick the earlier
//! one via `LocalResult::earliest()`.

use std::path::Path;

use chrono::{Local, NaiveDate, TimeZone};
use devlogger::entry::parse_file;

#[test]
fn ambiguous_dst_fallback_date_parses_cleanly() {
    // 2026-04-05 02:30:00 is ambiguous in Australia/Sydney (DST ends
    // 03:00 → 02:00 on that date).  In timezones without a DST-fallback
    // at this date the parse is trivially `Single` and the test still
    // asserts what it should: the parser must not error on this input.
    let naive = NaiveDate::from_ymd_opt(2026, 4, 5)
        .unwrap()
        .and_hms_opt(2, 30, 0)
        .unwrap();
    let resolution = Local.from_local_datetime(&naive);

    let contents = "- 1 | 2026-04-05 02:30:00: during dst fallback\n";
    let result = parse_file(Path::new("/fake/DEVLOG/main-devlog.md"), contents);

    match resolution {
        chrono::LocalResult::Ambiguous(..) | chrono::LocalResult::Single(..) => {
            let entries = result.expect("parse must succeed for ambiguous/single local time");
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].text, "during dst fallback");
        }
        chrono::LocalResult::None => {
            // Spring-forward gap in this TZ — this exact datetime cannot
            // exist here.  Parser is expected to error with a helpful
            // message.  Assert only that it *doesn't* use the old
            // "ambiguous" wording.
            let err = result.expect_err("impossible local time must error");
            let msg = format!("{err:#}");
            assert!(msg.contains("does not exist"), "msg: {msg}");
        }
    }
}

#[test]
fn unambiguous_date_still_parses_as_before() {
    // Sanity: the `.earliest()` swap must not break the common case.
    let contents = "- 1 | 2026-04-14 11:02:37: normal\n";
    let entries = parse_file(Path::new("/fake/DEVLOG/main-devlog.md"), contents).expect("parse ok");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].text, "normal");
}
