//! File I/O for devlog files.  Separates raw-bytes-on-disk from parsed
//! entries so update operations preserve user prose/headings/blank lines
//! and original line terminators.
//!
//! All state-mutating operations (`new`, `update`) must be performed while
//! holding a [`FileLock`] from [`acquire_lock_for`].  The sidecar lockfile
//! is a stable anchor for `flock(2)` — we deliberately don't lock the
//! devlog itself because `update` renames over it, which invalidates a
//! lock held on the old inode.

use eyre::{Result, WrapErr};
use fs2::FileExt;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::entry::{Entry, parse_file};

/// RAII handle for an exclusive advisory lock on a devlog's sidecar
/// lockfile.  Dropping the handle releases the lock.
#[must_use = "dropping the FileLock releases the exclusive lock"]
pub struct FileLock {
    // Keeping the File open keeps the flock active; the `_` prefix avoids
    // `dead_code` complaints while making intent clear.
    _file: File,
}

/// Path of the sidecar lockfile for a given devlog path:
/// `<devlog_dir>/.devlogger.lock`.
pub fn lock_path_for(devlog_path: &Path) -> Result<PathBuf> {
    let parent = devlog_path
        .parent()
        .ok_or_else(|| eyre::eyre!("devlog path {} has no parent", devlog_path.display()))?;
    Ok(parent.join(".devlogger.lock"))
}

/// Acquire an exclusive advisory lock for the given devlog path, creating
/// the devlog's parent directory and the sidecar lockfile if needed.  The
/// lock blocks until it can be acquired.
pub fn acquire_lock_for(devlog_path: &Path) -> Result<FileLock> {
    let parent = devlog_path
        .parent()
        .ok_or_else(|| eyre::eyre!("devlog path {} has no parent", devlog_path.display()))?;
    fs::create_dir_all(parent)
        .wrap_err_with(|| format!("failed to create directory {}", parent.display()))?;
    let lock_path = parent.join(".devlogger.lock");
    let file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .wrap_err_with(|| format!("failed to open lockfile {}", lock_path.display()))?;
    file.lock_exclusive()
        .wrap_err_with(|| format!("failed to acquire lock on {}", lock_path.display()))?;
    Ok(FileLock { _file: file })
}

/// Load all parsed entries from a devlog file.  Missing file returns
/// `Ok(vec![])` so `new` can create a log from scratch.
pub fn load_entries(path: &Path) -> Result<Vec<Entry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(path)
        .wrap_err_with(|| format!("failed to read devlog {}", path.display()))?;
    parse_file(path, &contents)
}

/// Read the raw file contents.  Propagates a clean error if missing.
pub fn read_contents(path: &Path) -> Result<String> {
    fs::read_to_string(path).wrap_err_with(|| format!("failed to read devlog {}", path.display()))
}

/// Append a single entry line to a devlog file, creating parent directories
/// and the file itself if needed.  Writes the full byte buffer (optional
/// leading terminator, line body, trailing terminator) as a **single**
/// `write_all`.  Splitting that across multiple syscalls (as `writeln!`
/// does via the fmt machinery) is a concurrency hazard even under
/// `O_APPEND`, since the line body and the newline can interleave with a
/// parallel writer.
///
/// The caller is responsible for holding an exclusive [`FileLock`]; this
/// function does not acquire one itself.
pub fn append_line(path: &Path, line: &str, line_ending: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("failed to create directory {}", parent.display()))?;
    }

    let existing = if path.exists() {
        fs::read_to_string(path).wrap_err_with(|| format!("failed to read {}", path.display()))?
    } else {
        String::new()
    };
    let needs_leading = !existing.is_empty() && !existing.ends_with('\n');

    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .wrap_err_with(|| format!("failed to open {}", path.display()))?;

    let mut buf: Vec<u8> = Vec::with_capacity(
        line.len() + line_ending.len() + if needs_leading { line_ending.len() } else { 0 },
    );
    if needs_leading {
        buf.extend_from_slice(line_ending.as_bytes());
    }
    buf.extend_from_slice(line.as_bytes());
    buf.extend_from_slice(line_ending.as_bytes());

    f.write_all(&buf)
        .wrap_err_with(|| format!("failed to write to {}", path.display()))?;
    Ok(())
}

/// Atomically rewrite a devlog file (tmp + rename).  Caller must hold the
/// [`FileLock`] since rename breaks flock semantics on the old inode.
pub fn rewrite_file(path: &Path, contents: &str) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| eyre::eyre!("devlog path {} has no parent directory", path.display()))?;
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("devlog");
    let tmp = parent.join(format!(".{file_name}.devlogger.tmp"));

    fs::write(&tmp, contents)
        .wrap_err_with(|| format!("failed to write tmp file {}", tmp.display()))?;
    fs::rename(&tmp, path)
        .wrap_err_with(|| format!("failed to rename {} to {}", tmp.display(), path.display()))?;
    Ok(())
}
