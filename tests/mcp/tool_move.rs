//! Tests for the `devlog_move` tool.

use rmcp::handler::server::wrapper::Parameters;

use super::common::*;
use devlogger::mcp::args::MoveArgs;

fn move_args(from: &str, id: &str, to: &str) -> Parameters<MoveArgs> {
    Parameters(MoveArgs {
        from_section: from.into(),
        id: id.into(),
        to_section: to.into(),
        base_dir: None,
    })
}

/// Seed two sections directly on disk so entry dates are deterministic
/// (going through `cmd_new` would stamp `Local::now()` and collide at
/// second resolution when the test runs fast).
fn seed_two_sections(
    base: &std::path::Path,
    from: &str,
    to: &str,
    from_entries: &[(&str, &str)],
    to_entries: &[(&str, &str)],
) {
    for (section, entries) in [(from, from_entries), (to, to_entries)] {
        let path = section_file(base, section);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut body = String::new();
        for (i, (date, text)) in entries.iter().enumerate() {
            body.push_str(&format!("- {} | {}: {}\n", i + 1, date, text));
        }
        std::fs::write(&path, body).unwrap();
    }
}

#[tokio::test]
async fn move_entry_appends_to_dest_when_newer_than_all_existing() {
    let (server, dir) = fresh_server();
    seed_two_sections(
        dir.path(),
        "from",
        "to",
        &[("2026-04-14 09:00:00", "moving me")],
        &[
            ("2026-04-10 09:00:00", "old one"),
            ("2026-04-12 09:00:00", "old two"),
        ],
    );

    let result = server
        .devlog_move(move_args("from", "1", "to"))
        .await
        .unwrap();
    assert_ok(&result);

    let s = structured(&result);
    assert_eq!(
        s.get("number").and_then(|v| v.as_u64()),
        Some(3),
        "should be #3 in dest"
    );
    assert_eq!(s.get("text").and_then(|v| v.as_str()), Some("moving me"));

    let to_contents = std::fs::read_to_string(section_file(dir.path(), "to")).unwrap();
    let entry_lines: Vec<&str> = to_contents
        .lines()
        .filter(|l| l.starts_with("- "))
        .collect();
    assert_eq!(entry_lines.len(), 3);
    assert!(entry_lines[0].starts_with("- 1 | 2026-04-10"));
    assert!(entry_lines[1].starts_with("- 2 | 2026-04-12"));
    assert!(entry_lines[2].starts_with("- 3 | 2026-04-14"));
    assert!(entry_lines[2].ends_with(": moving me"));
}

#[tokio::test]
async fn move_entry_inserts_in_middle_renumbering_trailing_entries() {
    let (server, dir) = fresh_server();
    seed_two_sections(
        dir.path(),
        "from",
        "to",
        &[("2026-04-13 09:00:00", "midpoint move")],
        &[
            ("2026-04-10 09:00:00", "dest a"),
            ("2026-04-15 09:00:00", "dest b"),
            ("2026-04-20 09:00:00", "dest c"),
        ],
    );

    let result = server
        .devlog_move(move_args("from", "1", "to"))
        .await
        .unwrap();
    assert_ok(&result);
    assert_eq!(
        structured(&result).get("number").and_then(|v| v.as_u64()),
        Some(2),
        "should slot between 2026-04-10 and 2026-04-15"
    );

    let to_contents = std::fs::read_to_string(section_file(dir.path(), "to")).unwrap();
    let entry_lines: Vec<&str> = to_contents
        .lines()
        .filter(|l| l.starts_with("- "))
        .collect();
    assert_eq!(entry_lines.len(), 4);
    assert!(entry_lines[0].starts_with("- 1 | 2026-04-10"));
    assert!(entry_lines[1].starts_with("- 2 | 2026-04-13"));
    assert!(entry_lines[1].ends_with(": midpoint move"));
    assert!(entry_lines[2].starts_with("- 3 | 2026-04-15"));
    assert!(entry_lines[3].starts_with("- 4 | 2026-04-20"));
}

#[tokio::test]
async fn move_entry_prepends_to_dest_when_older_than_all_existing() {
    let (server, dir) = fresh_server();
    seed_two_sections(
        dir.path(),
        "from",
        "to",
        &[("2026-04-01 09:00:00", "prehistoric")],
        &[
            ("2026-04-10 09:00:00", "dest a"),
            ("2026-04-15 09:00:00", "dest b"),
        ],
    );

    let result = server
        .devlog_move(move_args("from", "1", "to"))
        .await
        .unwrap();
    assert_ok(&result);
    assert_eq!(
        structured(&result).get("number").and_then(|v| v.as_u64()),
        Some(1)
    );

    let to_contents = std::fs::read_to_string(section_file(dir.path(), "to")).unwrap();
    let entry_lines: Vec<&str> = to_contents
        .lines()
        .filter(|l| l.starts_with("- "))
        .collect();
    assert_eq!(entry_lines.len(), 3);
    assert!(entry_lines[0].starts_with("- 1 | 2026-04-01"));
    assert!(entry_lines[0].ends_with(": prehistoric"));
    assert!(entry_lines[1].starts_with("- 2 | 2026-04-10"));
    assert!(entry_lines[2].starts_with("- 3 | 2026-04-15"));
}

#[tokio::test]
async fn move_entry_to_new_section_creates_it() {
    let (server, dir) = fresh_server();
    // Only create the source; leave the destination section absent.
    let from_path = section_file(dir.path(), "from");
    std::fs::create_dir_all(from_path.parent().unwrap()).unwrap();
    std::fs::write(&from_path, "- 1 | 2026-04-14 09:00:00: lone\n").unwrap();

    let result = server
        .devlog_move(move_args("from", "1", "fresh"))
        .await
        .unwrap();
    assert_ok(&result);
    assert_eq!(
        structured(&result).get("number").and_then(|v| v.as_u64()),
        Some(1)
    );

    let fresh_contents = std::fs::read_to_string(section_file(dir.path(), "fresh")).unwrap();
    assert!(fresh_contents.contains(": lone"));
    assert!(
        fresh_contents
            .lines()
            .any(|l| l.starts_with("- 1 | 2026-04-14"))
    );

    let from_contents = std::fs::read_to_string(from_path).unwrap();
    assert!(
        from_contents.lines().all(|l| !l.starts_with("- ")),
        "source should have no entry lines left, got {from_contents:?}"
    );
}

#[tokio::test]
async fn move_entry_renumbers_source_without_holes() {
    let (server, dir) = fresh_server();
    seed_two_sections(
        dir.path(),
        "from",
        "to",
        &[
            ("2026-04-10 09:00:00", "src a"),
            ("2026-04-11 09:00:00", "src b"),
            ("2026-04-12 09:00:00", "src c"),
            ("2026-04-13 09:00:00", "src d"),
        ],
        &[],
    );

    // Move the middle entry (2026-04-11, number 2).
    let result = server
        .devlog_move(move_args("from", "2", "to"))
        .await
        .unwrap();
    assert_ok(&result);

    let from_contents = std::fs::read_to_string(section_file(dir.path(), "from")).unwrap();
    let entry_lines: Vec<&str> = from_contents
        .lines()
        .filter(|l| l.starts_with("- "))
        .collect();
    assert_eq!(entry_lines.len(), 3, "one entry moved out");
    assert!(entry_lines[0].starts_with("- 1 | 2026-04-10"));
    assert!(entry_lines[0].ends_with(": src a"));
    assert!(entry_lines[1].starts_with("- 2 | 2026-04-12"));
    assert!(entry_lines[1].ends_with(": src c"));
    assert!(entry_lines[2].starts_with("- 3 | 2026-04-13"));
    assert!(entry_lines[2].ends_with(": src d"));
}

#[tokio::test]
async fn move_entry_by_exact_date_resolves_correctly() {
    let (server, dir) = fresh_server();
    seed_two_sections(
        dir.path(),
        "from",
        "to",
        &[
            ("2026-04-10 09:00:00", "wrong one"),
            ("2026-04-11 14:33:22", "target one"),
        ],
        &[],
    );

    let result = server
        .devlog_move(move_args("from", "2026-04-11 14:33:22", "to"))
        .await
        .unwrap();
    assert_ok(&result);
    assert_eq!(
        structured(&result).get("text").and_then(|v| v.as_str()),
        Some("target one")
    );
}

#[tokio::test]
async fn move_preserves_prose_in_both_sections() {
    let (server, dir) = fresh_server();
    let from_path = section_file(dir.path(), "from");
    let to_path = section_file(dir.path(), "to");
    std::fs::create_dir_all(from_path.parent().unwrap()).unwrap();
    std::fs::create_dir_all(to_path.parent().unwrap()).unwrap();

    // Source: prose between entries.
    std::fs::write(
        &from_path,
        "# from section\n\n\
         - 1 | 2026-04-10 09:00:00: keep me\n\
         \n\
         ## sub-heading\n\
         - 2 | 2026-04-14 09:00:00: moving out\n",
    )
    .unwrap();
    // Dest: prose before and after entries.
    std::fs::write(
        &to_path,
        "# to section\n\
         Some intro prose.\n\n\
         - 1 | 2026-04-01 09:00:00: existing\n\
         \n\
         Trailing notes.\n",
    )
    .unwrap();

    let result = server
        .devlog_move(move_args("from", "2", "to"))
        .await
        .unwrap();
    assert_ok(&result);

    let from_contents = std::fs::read_to_string(&from_path).unwrap();
    assert!(from_contents.contains("# from section"));
    assert!(from_contents.contains("## sub-heading"));
    assert!(from_contents.contains(": keep me"));
    assert!(!from_contents.contains(": moving out"));

    let to_contents = std::fs::read_to_string(&to_path).unwrap();
    assert!(to_contents.contains("# to section"));
    assert!(to_contents.contains("Some intro prose."));
    assert!(to_contents.contains("Trailing notes."));
    assert!(to_contents.contains(": moving out"));
    // Dest now has two entries: "existing" (2026-04-01) first, "moving out"
    // (2026-04-14) second, numbered 1 and 2.
    let entry_lines: Vec<&str> = to_contents
        .lines()
        .filter(|l| l.starts_with("- "))
        .collect();
    assert_eq!(entry_lines.len(), 2);
    assert!(entry_lines[0].starts_with("- 1 | 2026-04-01"));
    assert!(entry_lines[1].starts_with("- 2 | 2026-04-14"));
}

#[tokio::test]
async fn move_to_same_section_is_tool_error() {
    let (server, dir) = fresh_server();
    seed_two_sections(
        dir.path(),
        "core",
        "other",
        &[("2026-04-14 09:00:00", "x")],
        &[],
    );

    let result = server
        .devlog_move(move_args("core", "1", "core"))
        .await
        .unwrap();
    let msg = assert_err(&result);
    assert!(msg.contains("same section"), "got {msg}");
}

#[tokio::test]
async fn move_missing_source_section_is_tool_error() {
    let (server, _dir) = fresh_server();
    let result = server
        .devlog_move(move_args("nope", "1", "elsewhere"))
        .await
        .unwrap();
    let msg = assert_err(&result);
    assert!(msg.contains("not found"), "got {msg}");
}

#[tokio::test]
async fn move_unknown_id_is_tool_error() {
    let (server, dir) = fresh_server();
    seed_two_sections(
        dir.path(),
        "from",
        "to",
        &[("2026-04-14 09:00:00", "only")],
        &[],
    );
    let result = server
        .devlog_move(move_args("from", "99", "to"))
        .await
        .unwrap();
    let msg = assert_err(&result);
    assert!(msg.contains("no entry"), "got {msg}");
}

#[tokio::test]
async fn move_rejects_invalid_section_name_as_tool_error() {
    let (server, dir) = fresh_server();
    seed_two_sections(
        dir.path(),
        "from",
        "to",
        &[("2026-04-14 09:00:00", "x")],
        &[],
    );
    let result = server
        .devlog_move(move_args("from", "1", "Bad"))
        .await
        .unwrap();
    let msg = assert_err(&result);
    assert!(
        msg.contains("invalid section") || msg.contains("illegal"),
        "got {msg}"
    );
}

#[tokio::test]
async fn move_preserves_original_date_on_moved_entry() {
    let (server, dir) = fresh_server();
    seed_two_sections(
        dir.path(),
        "from",
        "to",
        &[("2026-03-07 11:22:33", "datestamp")],
        &[],
    );

    let result = server
        .devlog_move(move_args("from", "1", "to"))
        .await
        .unwrap();
    assert_ok(&result);
    assert_eq!(
        structured(&result).get("date").and_then(|v| v.as_str()),
        Some("2026-03-07 11:22:33"),
        "date must be preserved verbatim"
    );

    let to_contents = std::fs::read_to_string(section_file(dir.path(), "to")).unwrap();
    assert!(to_contents.contains("2026-03-07 11:22:33"));
}
