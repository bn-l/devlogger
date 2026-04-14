//! Devlog entry format: `- <number> | <date>: <entry>`
//!
//! The parser is strict on `- ` lines (they must match this shape) and
//! permissive on every other line (so users can add prose, headings, and
//! blank lines to the markdown file without corrupting it).  Parse errors
//! report the file path, the 1-based line number, and a short description
//! of what was wrong — never the file's contents.

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use eyre::{Result, bail};
use std::fmt;
use std::path::{Path, PathBuf};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Date format used in entry lines.
pub const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// Reject entry text that would break the single-line format: no newlines,
/// no carriage returns, and no control characters other than tab.
pub fn validate_entry_text(text: &str) -> Result<()> {
    for (i, ch) in text.char_indices() {
        match ch {
            '\n' => bail!("entry text must not contain a newline (byte {i})"),
            '\r' => bail!("entry text must not contain a carriage return (byte {i})"),
            c if c.is_control() && c != '\t' => bail!(
                "entry text must not contain control character U+{:04X} (byte {i})",
                c as u32
            ),
            _ => {}
        }
    }
    Ok(())
}

/// Detect the line terminator style of a file's contents.  Returns `"\r\n"`
/// if the first terminator found is CRLF, else `"\n"`.  Empty files and
/// files with no terminator default to `"\n"`.
pub fn detect_line_ending(contents: &str) -> &'static str {
    let b = contents.as_bytes();
    for i in 0..b.len() {
        if b[i] == b'\n' {
            return if i > 0 && b[i - 1] == b'\r' {
                "\r\n"
            } else {
                "\n"
            };
        }
    }
    "\n"
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub number: u32,
    pub date: DateTime<Local>,
    pub text: String,
}

impl Entry {
    pub fn new(number: u32, date: DateTime<Local>, text: impl Into<String>) -> Self {
        Self {
            number,
            date,
            text: text.into(),
        }
    }

    /// Render as a devlog line (no trailing newline).
    pub fn to_line(&self) -> String {
        format!(
            "- {} | {}: {}",
            self.number,
            self.date.format(DATE_FORMAT),
            self.text,
        )
    }

    /// Render as a devlog line truncated to at most `max_cols` terminal
    /// display columns, including the ` (...N more)` suffix when elision
    /// occurs.  Wide glyphs (CJK, most emoji) count as 2 columns, so this
    /// genuinely fits in a terminal of that width.
    pub fn to_line_truncated(&self, max_cols: usize) -> String {
        truncate_line(&self.to_line(), max_cols)
    }
}

/// Truncate a line to at most `max_cols` terminal display columns.  If the
/// line is longer, keep a prefix and append ` (...N more)` where `N` is the
/// number of elided **characters** (Unicode code points) — preserving the
/// user-visible contract from the command help.  The returned string's
/// display width is always `<= max_cols`, never exceeding it even for
/// wide glyphs.  Characters that would straddle the budget are dropped
/// (we never emit a partial glyph).
pub fn truncate_line(line: &str, max_cols: usize) -> String {
    if line.width() <= max_cols {
        return line.to_string();
    }
    let total_chars = line.chars().count();

    // Suffix is ` (...N more)` — ASCII only, so its display width equals
    // its byte/char count: 11 + digit_count(N).  N is elided *characters*,
    // which depends on how many chars we keep — which depends on suffix
    // width — which depends on digit count of N.  Iterate to stabilize:
    // as kept grows, N shrinks, digit count is non-increasing, so this
    // converges in at most a handful of steps.
    let suffix_overhead = " (... more)".chars().count(); // = 11
    let mut suffix_digits = digit_count(total_chars);
    for _ in 0..8 {
        let budget = max_cols.saturating_sub(suffix_overhead + suffix_digits);
        let kept_chars = longest_prefix_within(line, budget);
        let elided = total_chars - kept_chars;
        let new_digits = digit_count(elided);
        if new_digits == suffix_digits {
            let prefix: String = line.chars().take(kept_chars).collect();
            return format!("{prefix} (...{elided} more)");
        }
        suffix_digits = new_digits;
    }
    // Defensive: if we somehow fail to stabilize, emit something
    // reasonable using the final estimate.
    let budget = max_cols.saturating_sub(suffix_overhead + suffix_digits);
    let kept_chars = longest_prefix_within(line, budget);
    let elided = total_chars - kept_chars;
    let prefix: String = line.chars().take(kept_chars).collect();
    format!("{prefix} (...{elided} more)")
}

/// Return the length (in Unicode characters) of the longest prefix of
/// `line` whose terminal display width is `<= budget`.
fn longest_prefix_within(line: &str, budget: usize) -> usize {
    let mut width = 0usize;
    let mut chars = 0usize;
    for c in line.chars() {
        let cw = c.width().unwrap_or(0);
        if width + cw > budget {
            break;
        }
        width += cw;
        chars += 1;
    }
    chars
}

fn digit_count(mut n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let mut d = 0;
    while n > 0 {
        d += 1;
        n /= 10;
    }
    d
}

/// Succinct parse error — path + line + reason.  Never includes file
/// contents, which would be noisy garbage in a terminal.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub path: PathBuf,
    pub line: usize,
    pub reason: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "devlog parse error at {}:{}: {}; expected `- <number> | <YYYY-MM-DD HH:MM:SS>: <entry>`",
            self.path.display(),
            self.line,
            self.reason,
        )
    }
}

impl std::error::Error for ParseError {}

/// Parse every `- ` line in the file contents.  Lines that do NOT start
/// with `- ` are intentionally ignored (they're prose, blank lines, etc.).
/// Lines that DO start with `- ` must match the canonical entry format or
/// a `ParseError` is returned.
pub fn parse_file(path: &Path, contents: &str) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for (idx, raw_line) in contents.lines().enumerate() {
        let line_no = idx + 1;
        if let Some(rest) = raw_line.strip_prefix("- ") {
            let entry = parse_entry_line(path, line_no, rest)?;
            entries.push(entry);
        }
    }
    Ok(entries)
}

fn parse_entry_line(path: &Path, line_no: usize, rest: &str) -> Result<Entry, ParseError> {
    let make_err = |reason: String| ParseError {
        path: path.to_path_buf(),
        line: line_no,
        reason,
    };

    let (num_str, after_num) = rest
        .split_once(" | ")
        .ok_or_else(|| make_err("missing ` | ` separator between number and date".into()))?;

    let num_str = num_str.trim();
    let number: u32 = num_str
        .parse()
        .map_err(|_| make_err(format!("entry number `{num_str}` is not a positive integer")))?;

    let (date_str, text) = after_num
        .split_once(": ")
        .ok_or_else(|| make_err("missing `: ` separator between date and entry text".into()))?;

    let date_str = date_str.trim();
    let naive = NaiveDateTime::parse_from_str(date_str, DATE_FORMAT).map_err(|_| {
        make_err(format!(
            "date `{date_str}` is not in YYYY-MM-DD HH:MM:SS format"
        ))
    })?;

    // `earliest()` handles both the normal `Single` case and the DST
    // fall-back `Ambiguous` case (picks the earlier of the two instants
    // deterministically, since the naive on-disk format cannot
    // distinguish them).  Only returns `None` for an impossible local
    // time during the DST spring-forward gap — which we surface as a
    // parse error since no real `Local::now()` could ever have produced
    // such a value.
    let date = Local
        .from_local_datetime(&naive)
        .earliest()
        .ok_or_else(|| {
            make_err(format!(
                "date `{date_str}` does not exist in the local timezone (DST spring-forward gap)"
            ))
        })?;

    Ok(Entry::new(number, date, text))
}
