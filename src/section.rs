//! Section name validation and path resolution.
//!
//! A section name must match `^[a-z]+(-[a-z]+)*$`: lowercase letters a-z
//! and hyphens only, non-empty, no leading/trailing hyphens, no consecutive
//! hyphens.  Digits, uppercase, underscores, whitespace, etc. are rejected
//! at creation time — not silently coerced or accepted.

use eyre::{Result, bail};
use std::path::{Path, PathBuf};

/// Validate a section name.  Returns `Ok(())` if valid; otherwise an error
/// whose Display explains precisely what's wrong.
pub fn validate_section_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("invalid section name: empty string");
    }

    let chars: Vec<char> = name.chars().collect();

    if chars[0] == '-' {
        bail!("invalid section name '{name}': must not start with '-'");
    }
    if *chars.last().expect("non-empty") == '-' {
        bail!("invalid section name '{name}': must not end with '-'");
    }

    let mut prev_hyphen = false;
    for (i, ch) in chars.iter().enumerate() {
        match ch {
            'a'..='z' => prev_hyphen = false,
            '-' => {
                if prev_hyphen {
                    bail!(
                        "invalid section name '{name}': consecutive hyphens at position {i}"
                    );
                }
                prev_hyphen = true;
            }
            other => bail!(
                "invalid section name '{name}': illegal character '{other}' at position {i} (allowed: a-z and '-')"
            ),
        }
    }

    Ok(())
}

/// `<base>/DEVLOG/<section>/<section>-devlog.md`.  The caller must have
/// validated the section name first.
pub fn section_devlog_path(base: &Path, section: &str) -> PathBuf {
    base.join("DEVLOG")
        .join(section)
        .join(format!("{section}-devlog.md"))
}
