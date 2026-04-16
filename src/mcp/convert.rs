//! Shared conversions between devlogger's internal types and the JSON
//! shapes exposed through MCP tool results.

use serde::Serialize;

use crate::entry::{DATE_FORMAT, Entry};

/// JSON projection of a single devlog entry.  Stable wire shape — do not
/// rename fields without thinking through client compatibility.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EntryJson {
    pub number: u32,
    /// `YYYY-MM-DD HH:MM:SS` in the server's local timezone, matching the
    /// on-disk format.
    pub date: String,
    pub text: String,
    /// Full canonical line, i.e. `- <number> | <date>: <text>`.
    pub line: String,
}

impl From<&Entry> for EntryJson {
    fn from(e: &Entry) -> Self {
        Self {
            number: e.number,
            date: e.date.format(DATE_FORMAT).to_string(),
            text: e.text.clone(),
            line: e.to_line(),
        }
    }
}

impl From<Entry> for EntryJson {
    fn from(e: Entry) -> Self {
        Self::from(&e)
    }
}

/// Convert a slice of entries to their JSON projections.
pub fn entries_to_json(entries: &[Entry]) -> Vec<EntryJson> {
    entries.iter().map(EntryJson::from).collect()
}

/// JSON projection of a section's entries, used for `devlog_list` without
/// a section.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SectionEntriesJson {
    pub section: String,
    pub entries: Vec<EntryJson>,
}
