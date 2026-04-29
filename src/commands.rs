//! Command handlers: `new`, `list`, `update`, `read`.
//!
//! These are the behaviour-bearing functions; `main.rs` is a thin parser
//! that dispatches positional args into these.  All state-mutating
//! commands acquire an exclusive advisory file lock (see
//! [`crate::store::acquire_lock_for`]) before reading and hold it until
//! the write completes, so concurrent invocations are serialized.

use chrono::Local;
use eyre::{Result, WrapErr, bail};
use std::fs;
use std::path::{Path, PathBuf};

use crate::entry::{DATE_FORMAT, Entry, detect_line_ending, parse_file, validate_entry_text};
use crate::section::{section_devlog_path, validate_section_name};
use crate::store::{acquire_lock_for, append_line, load_entries, read_contents, rewrite_file};

/// Resolve the devlog file path for a section.  Validates the section name.
pub fn resolve_path(base: &Path, section: &str) -> Result<PathBuf> {
    validate_section_name(section)?;
    Ok(section_devlog_path(base, section))
}

/// Append a new entry.  Number is max(existing) + 1, or 1 if empty.
/// Holds an exclusive lock across the read-compute-write so parallel
/// invocations cannot produce duplicate numbers or interleaved writes.
pub fn cmd_new(base: &Path, section: &str, text: &str) -> Result<Entry> {
    let path = prepare_new_path(base, section, text)?;
    cmd_new_prevalidated(&path, text)
}

/// Validate the cheap, CPU-only `new` inputs and return the canonical
/// devlog path.  MCP handlers call this before entering blocking file
/// work so validation errors cannot queue behind file locks or I/O.
pub fn prepare_new_path(base: &Path, section: &str, text: &str) -> Result<PathBuf> {
    validate_entry_text(text)?;
    resolve_path(base, section)
}

/// Append a new entry after `section` and `text` have already been
/// validated.  This is the blocking file-work part of [`cmd_new`].
pub fn cmd_new_prevalidated(path: &Path, text: &str) -> Result<Entry> {
    let _lock = acquire_lock_for(path)?;

    let existing_contents = if path.exists() {
        read_contents(path)?
    } else {
        String::new()
    };
    let existing = parse_file(path, &existing_contents)?;
    let next_number = match existing.iter().map(|e| e.number).max() {
        None => 1,
        Some(max) => max.checked_add(1).ok_or_else(|| {
            eyre::eyre!(
                "entry numbering exhausted: {} already has an entry numbered {} (u32::MAX)",
                path.display(),
                u32::MAX,
            )
        })?,
    };

    let line_ending = detect_line_ending(&existing_contents);

    let entry = Entry::new(next_number, Local::now(), text);
    append_line(path, &entry.to_line(), line_ending)?;

    Ok(entry)
}

/// Load all entries from a section's devlog.
pub fn cmd_list(base: &Path, section: &str) -> Result<Vec<Entry>> {
    let path = resolve_path(base, section)?;
    if !path.exists() {
        bail!("devlog not found: {}", path.display());
    }
    load_entries(&path)
}

/// Load entries for every section, returning a vector of (section, entries)
/// pairs in alphabetical order of section name.  Sections with no parseable
/// devlog file are skipped — `cmd_sections` already filters to sections
/// whose canonical file exists.
pub fn cmd_list_all(base: &Path) -> Result<Vec<(String, Vec<Entry>)>> {
    let sections = cmd_sections(base)?;
    let mut out = Vec::with_capacity(sections.len());
    for name in sections {
        let entries = load_entries(&section_devlog_path(base, &name))?;
        out.push((name, entries));
    }
    Ok(out)
}

/// List every section that has a devlog under `<base>/DEVLOG/`.  A section
/// counts only when both its directory name is a valid section name AND
/// the canonical `<name>/<name>-devlog.md` file exists — stray
/// directories under `DEVLOG/` are ignored.  Names are returned sorted
/// alphabetically.  A missing `DEVLOG/` directory returns an empty vector
/// rather than an error, so `sections` on a fresh project is a no-op
/// instead of a failure.
pub fn cmd_sections(base: &Path) -> Result<Vec<String>> {
    let devlog_dir = base.join("DEVLOG");
    if !devlog_dir.exists() {
        return Ok(Vec::new());
    }
    let read_dir = fs::read_dir(&devlog_dir)
        .wrap_err_with(|| format!("failed to read {}", devlog_dir.display()))?;

    let mut sections = Vec::new();
    for dirent in read_dir {
        let dirent = dirent.wrap_err_with(|| format!("failed to read {}", devlog_dir.display()))?;
        let path = dirent.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if validate_section_name(name).is_err() {
            continue;
        }
        if path.join(format!("{name}-devlog.md")).is_file() {
            sections.push(name.to_string());
        }
    }
    sections.sort();
    Ok(sections)
}

/// Update an existing entry's text.  `id` is either:
///   * a number matching the entry's canonical `number` field, or
///   * an exact `YYYY-MM-DD HH:MM:SS` date string, or
///   * a date prefix (e.g. `2026-04-14`) that uniquely matches one entry.
///
/// The entry's number and date are preserved; only the text changes.
/// The target is identified by its *position* among entry-shaped lines,
/// not by re-matching its number — so duplicate numbers in a
/// hand-edited file do not cause the wrong line to be rewritten.
/// Non-entry lines (prose, blanks, headings) are preserved, as are the
/// file's original line terminators (`\n` or `\r\n`).
pub fn cmd_update(base: &Path, section: &str, id: &str, new_text: &str) -> Result<Entry> {
    let path = prepare_update_path(base, section, new_text)?;
    cmd_update_prevalidated(&path, id, new_text)
}

/// Validate the cheap, CPU-only `update` inputs and return the canonical
/// devlog path.  Existence/read/parse/rewrite remain in the blocking
/// section because they touch the filesystem.
pub fn prepare_update_path(base: &Path, section: &str, new_text: &str) -> Result<PathBuf> {
    validate_entry_text(new_text)?;
    resolve_path(base, section)
}

/// Update an entry after `section` and replacement text have already
/// been validated.  This is the blocking file-work part of [`cmd_update`].
pub fn cmd_update_prevalidated(path: &Path, id: &str, new_text: &str) -> Result<Entry> {
    if !path.exists() {
        bail!("devlog not found: {}", path.display());
    }
    let _lock = acquire_lock_for(path)?;

    let contents = read_contents(path)?;
    let entries = parse_file(path, &contents)?;
    let target_idx = resolve_target(&entries, id)?;
    let target = entries[target_idx].clone();

    let new_line = format!(
        "- {} | {}: {}",
        target.number,
        target.date.format(DATE_FORMAT),
        new_text,
    );

    let line_ending = detect_line_ending(&contents);
    let trailing_newline = contents.ends_with('\n');

    let mut out = String::with_capacity(contents.len() + new_text.len());
    let mut entry_counter: usize = 0;
    let mut replaced = false;
    for line in contents.lines() {
        if line.starts_with("- ") {
            if entry_counter == target_idx {
                out.push_str(&new_line);
                replaced = true;
            } else {
                out.push_str(line);
            }
            entry_counter += 1;
        } else {
            out.push_str(line);
        }
        out.push_str(line_ending);
    }
    if !trailing_newline && out.ends_with(line_ending) {
        out.truncate(out.len() - line_ending.len());
    }

    if !replaced {
        bail!(
            "internal error: target index {} not found while rewriting {}",
            target_idx,
            path.display()
        );
    }

    rewrite_file(&path, &out).wrap_err("failed to rewrite devlog")?;

    Ok(Entry::new(target.number, target.date, new_text))
}

/// Move an entry from one section to another.  The entry's date is
/// preserved; it is inserted into the destination at its correct
/// chronological position and both sections are renumbered 1..N so that
/// number order matches file (and for the common append-only case, date)
/// order.  Crash safety: the destination is written first (so a crash
/// mid-operation leaves a duplicate, which is visible and fixable, rather
/// than silently losing the entry).  The returned [`Entry`] reflects the
/// entry's new number in the destination.
pub fn cmd_move(base: &Path, from_section: &str, id: &str, to_section: &str) -> Result<Entry> {
    validate_section_name(from_section)?;
    validate_section_name(to_section)?;
    if from_section == to_section {
        bail!(
            "cannot move within the same section (`{from_section}`); \
             use `update` if you want to rewrite an entry's text"
        );
    }
    let from_path = section_devlog_path(base, from_section);
    let to_path = section_devlog_path(base, to_section);
    if !from_path.exists() {
        bail!("devlog not found: {}", from_path.display());
    }

    // Lock both sections, acquiring in alphabetical order so two
    // concurrent moves in opposite directions can never deadlock.
    let (_first_lock, _second_lock) = if from_section < to_section {
        let a = acquire_lock_for(&from_path)?;
        let b = acquire_lock_for(&to_path)?;
        (a, b)
    } else {
        let b = acquire_lock_for(&to_path)?;
        let a = acquire_lock_for(&from_path)?;
        (a, b)
    };

    let from_contents = read_contents(&from_path)?;
    let from_entries = parse_file(&from_path, &from_contents)?;
    let target_idx = resolve_target(&from_entries, id)?;
    let moved = from_entries[target_idx].clone();

    let to_contents = if to_path.exists() {
        read_contents(&to_path)?
    } else {
        String::new()
    };
    let to_entries = parse_file(&to_path, &to_contents)?;

    // Insertion point in dest = first existing entry whose date is
    // strictly greater than the moved entry's.  Ties break toward
    // "after", so moves of same-second entries are stable.
    let insertion_pos = to_entries
        .iter()
        .position(|e| e.date > moved.date)
        .unwrap_or(to_entries.len());

    let new_to_contents =
        build_contents_with_insert(&to_contents, &to_entries, insertion_pos, &moved);
    let new_from_contents =
        build_contents_without_target(&from_contents, &from_entries, target_idx);

    // Dest first (add), then source (remove): crash between writes
    // leaves a duplicate, which is visible and recoverable.
    rewrite_file(&to_path, &new_to_contents)
        .wrap_err_with(|| format!("failed to rewrite {}", to_path.display()))?;
    rewrite_file(&from_path, &new_from_contents)
        .wrap_err_with(|| format!("failed to rewrite {}", from_path.display()))?;

    let new_number = (insertion_pos as u32) + 1;
    Ok(Entry::new(new_number, moved.date, moved.text))
}

/// Build the destination file contents with `moved` inserted at
/// `insertion_pos` (0-based index among existing entries).  All entries
/// are renumbered 1..N by their file-position order, so number order
/// matches file order after the move.  Prose, headings, blank lines, and
/// the file's original line-ending style are preserved.
fn build_contents_with_insert(
    contents: &str,
    entries: &[Entry],
    insertion_pos: usize,
    moved: &Entry,
) -> String {
    let line_ending = detect_line_ending(contents);

    // Empty/degenerate file: the moved entry becomes the sole entry.
    if entries.is_empty() {
        let line = format_entry_line(1, moved);
        if contents.is_empty() {
            return format!("{line}{line_ending}");
        }
        // File has prose but no entries — append the moved entry after
        // the existing prose, preserving the file's trailing-newline
        // policy.
        let mut out = contents.to_string();
        if !out.ends_with('\n') {
            out.push_str(line_ending);
        }
        out.push_str(&line);
        out.push_str(line_ending);
        if !contents.ends_with('\n') {
            out.truncate(out.len() - line_ending.len());
        }
        return out;
    }

    let trailing_newline = contents.ends_with('\n');
    let lines: Vec<&str> = contents.lines().collect();
    let last_entry_line_idx = lines
        .iter()
        .rposition(|l| l.starts_with("- "))
        .expect("entries.is_empty() handled above");

    let mut out = String::with_capacity(contents.len() + 64);
    let mut existing_seen = 0usize;
    let mut next_number: u32 = 1;
    let mut inserted = false;

    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("- ") {
            // Insert BEFORE this entry line if it's the insertion point.
            if !inserted && existing_seen == insertion_pos {
                out.push_str(&format_entry_line(next_number, moved));
                out.push_str(line_ending);
                next_number += 1;
                inserted = true;
            }
            out.push_str(&format_entry_line(next_number, &entries[existing_seen]));
            out.push_str(line_ending);
            next_number += 1;
            existing_seen += 1;
        } else {
            out.push_str(line);
            out.push_str(line_ending);
        }

        // Insert AFTER the last entry line if we still haven't placed
        // the new entry (i.e. insertion_pos == entries.len()).
        if !inserted && i == last_entry_line_idx {
            out.push_str(&format_entry_line(next_number, moved));
            out.push_str(line_ending);
            next_number += 1;
            inserted = true;
        }
    }

    debug_assert!(inserted, "insertion point never reached");

    if !trailing_newline && out.ends_with(line_ending) {
        out.truncate(out.len() - line_ending.len());
    }
    out
}

/// Build the source file contents with the entry at `target_idx` removed.
/// Remaining entries are renumbered 1..N by file-position order (no
/// gaps).  Prose and line endings are preserved.
fn build_contents_without_target(contents: &str, entries: &[Entry], target_idx: usize) -> String {
    let line_ending = detect_line_ending(contents);
    let trailing_newline = contents.ends_with('\n');

    let mut out = String::with_capacity(contents.len());
    let mut existing_seen = 0usize;
    let mut next_number: u32 = 1;
    let mut removed = false;

    for line in contents.lines() {
        if line.starts_with("- ") {
            if existing_seen == target_idx {
                existing_seen += 1;
                removed = true;
                continue;
            }
            out.push_str(&format_entry_line(next_number, &entries[existing_seen]));
            out.push_str(line_ending);
            next_number += 1;
            existing_seen += 1;
        } else {
            out.push_str(line);
            out.push_str(line_ending);
        }
    }

    debug_assert!(removed, "target index {target_idx} never encountered");

    if !trailing_newline && out.ends_with(line_ending) {
        out.truncate(out.len() - line_ending.len());
    }
    out
}

fn format_entry_line(number: u32, entry: &Entry) -> String {
    format!(
        "- {} | {}: {}",
        number,
        entry.date.format(DATE_FORMAT),
        entry.text,
    )
}

/// Read the devlog.  `n = None` dumps the whole file verbatim.  `n =
/// Some(k)` dumps the last `k` entry lines (prose lines are skipped).
pub fn cmd_read(base: &Path, section: &str, n: Option<usize>) -> Result<String> {
    let path = resolve_path(base, section)?;
    if !path.exists() {
        bail!("devlog not found: {}", path.display());
    }
    let contents = read_contents(&path)?;

    match n {
        None => Ok(contents),
        Some(k) => {
            let _ = parse_file(&path, &contents)?;
            let lines: Vec<&str> = contents.lines().filter(|l| l.starts_with("- ")).collect();
            let start = lines.len().saturating_sub(k);
            let mut out = lines[start..].join("\n");
            if !out.is_empty() {
                out.push('\n');
            }
            Ok(out)
        }
    }
}

/// Find the entry matching a CLI-supplied id, returning its **index** into
/// `entries`.  Resolution order: number; exact date; date prefix.  When a
/// numeric id matches multiple entries (possible in a hand-edited file),
/// we refuse to guess and ask the user to disambiguate by date.
fn resolve_target(entries: &[Entry], id: &str) -> Result<usize> {
    if let Ok(n) = id.parse::<u32>() {
        let matches: Vec<usize> = entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.number == n)
            .map(|(i, _)| i)
            .collect();
        return match matches.len() {
            0 => bail!("no entry with number {n}"),
            1 => Ok(matches[0]),
            k => bail!(
                "ambiguous: {k} entries share number {n}; use the exact date shown by `list` to disambiguate"
            ),
        };
    }

    let exact: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.date.format(DATE_FORMAT).to_string() == id)
        .map(|(i, _)| i)
        .collect();
    if exact.len() == 1 {
        return Ok(exact[0]);
    }
    if exact.len() > 1 {
        bail!(
            "ambiguous date `{id}`: {} entries share that exact timestamp",
            exact.len()
        );
    }

    let prefix: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.date.format(DATE_FORMAT).to_string().starts_with(id))
        .map(|(i, _)| i)
        .collect();
    match prefix.len() {
        0 => bail!("no entry matches id `{id}` (not a known number, not a matching date)"),
        1 => Ok(prefix[0]),
        _ => {
            let nums: Vec<String> = prefix
                .iter()
                .map(|&i| entries[i].number.to_string())
                .collect();
            bail!(
                "ambiguous date prefix `{id}`: matches entries numbered {}. Use the full date shown by `list`.",
                nums.join(", ")
            )
        }
    }
}
