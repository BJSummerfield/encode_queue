//! Environment-derived configuration values (the config boundary).
//!
//! `QueuePath` and `Editor` are both produced from `OsString` env values via
//! `TryFrom`, so user-supplied values and the `"nvim"` default are validated
//! identically.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::error::DomainError;

/// The validated queue-file location (`~/.config/encode_queue/commands.txt`).
/// Wraps the validated UTF-8 path string, so `as_str()` is infallible and the
/// old `to_str().unwrap()` panic (C004) is structurally impossible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuePath(String);

impl QueuePath {
    /// The queue file path as `&str` (used for the editor arg).
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The queue file path as `&Path` (used for file I/O).
    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }

    /// The queue directory (`~/.config/encode_queue`), for `create_dir_all`.
    /// `parent()` is present for our absolute path; the `unwrap_or` fallback is
    /// a total (non-panicking) safety net, not an `unwrap` (C004).
    pub fn dir(&self) -> &Path {
        Path::new(&self.0).parent().unwrap_or_else(|| Path::new(&self.0))
    }
}

impl TryFrom<OsString> for QueuePath {
    type Error = DomainError;

    /// Build the queue path from a validated `$HOME` (presence already checked
    /// by the caller). Validates UTF-8 and assembles the canonical path.
    fn try_from(home: OsString) -> Result<Self, Self::Error> {
        let home = home.into_string().map_err(DomainError::HomeNotUtf8)?;
        let file = PathBuf::from(home)
            .join(".config/encode_queue/commands.txt")
            .into_os_string();
        let file = file.into_string().map_err(DomainError::QueuePathNotUtf8)?;
        Ok(Self(file))
    }
}

/// The wrapped value verbatim.
impl std::fmt::Display for QueuePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Access the wrapped value as `&str`.
impl std::convert::AsRef<str> for QueuePath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// The validated editor program name (`$EDITOR`, or the `"nvim"` default).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Editor(String);

impl Editor {
    /// The editor program name as validated and stored.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<OsString> for Editor {
    type Error = DomainError;

    /// Validate the editor program name is UTF-8. The `"nvim"` default is
    /// routed through this same conversion so it is validated too.
    fn try_from(value: OsString) -> Result<Self, Self::Error> {
        let s = value.into_string().map_err(DomainError::EditorNotUtf8)?;
        Ok(Self(s))
    }
}

/// The wrapped value verbatim.
impl std::fmt::Display for Editor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Access the wrapped value as `&str`.
impl std::convert::AsRef<str> for Editor {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
