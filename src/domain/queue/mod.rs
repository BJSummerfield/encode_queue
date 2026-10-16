//! Queue domain newtypes and their canonical error.
//!
//! Scaffold (WP01): the newtypes are shells (private field + a constructor +
//! the accessors and `Display` that downstream work packages code against) and
//! `QueueEntryError` is a skeleton; its variant set and the strict constructor
//! behaviour are finalized in WP02. No third-party imports here (ARCH-01).

use std::fmt;
use std::path::{Path, PathBuf};

/// A single queued command line.
///
/// Scaffold constructor; the lenient verbatim store is preserved so that
/// `add`/`ls` stay byte-for-byte unchanged (the strict blank rejection is a
/// deferred P5/P6 behaviour change).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEntry(String);

impl QueueEntry {
    /// Canonical constructor: stores the raw command line verbatim.
    pub fn new(line: &str) -> Self {
        Self(line.to_owned())
    }

    /// Borrow the raw command line.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for QueueEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The location of the queue file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueuePath(PathBuf);

impl QueuePath {
    /// Wrap a queue-file path.
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    /// Borrow the wrapped path.
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl fmt::Display for QueuePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

/// The editor binary to launch on `edit`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Editor(String);

impl Editor {
    /// Wrap an editor name.
    pub fn new(name: &str) -> Self {
        Self(name.to_owned())
    }

    /// Borrow the editor name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Editor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Errors surfaced by the queue domain.
///
/// Skeleton (WP01): the `thiserror` derive is in place; the canonical `Empty`
/// variant and the strict validation that would produce it land in WP02.
#[derive(Debug, thiserror::Error)]
pub enum QueueEntryError {}
