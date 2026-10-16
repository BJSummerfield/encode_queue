//! Queue domain newtypes and their canonical error.
//!
//! WP02: the newtypes are finalized per TY-01..TY-06 — private fields,
//! canonical fallible constructors with one-line `FromStr`/`TryFrom`
//! delegations (TY-04), named accessors plus `Display` (no `Deref`/`Borrow`,
//! TY-05), and `#[cfg(test)]` per-constructor tests (TST-02). No
//! third-party imports here (ARCH-01).
//!
//! **Lenient by design (behaviour preservation).** `QueueEntry`'s
//! constructor stores the raw line **verbatim** (no trim) and accepts
//! empty/whitespace-only input so `add`/`ls` stay byte-for-byte unchanged.
//! The strict trim + `QueueEntryError::Empty` rejection that TY-01's
//! canonical `QueueEntry` shows is a **deferred** P5/P6 behaviour change:
//! the `Empty` variant is declared as the domain's public error type for
//! that path but is not produced on the P5 path.

pub mod service;

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// A single queued command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEntry(String);

impl QueueEntry {
    /// Canonical constructor: stores the raw command line **verbatim**.
    ///
    /// Lenient by design (behaviour preservation): no trim, no blank
    /// validation. Empty and whitespace-only input is a valid entry stored
    /// exactly as given, so `add`/`ls` remain byte-for-byte unchanged. This
    /// arm therefore never fails on the P5 path; the strict trim +
    /// [`QueueEntryError::Empty`] reject is the deferred P5/P6 behaviour
    /// change and is not implemented here.
    pub fn new(line: &str) -> Result<Self, QueueEntryError> {
        Ok(Self(line.to_owned()))
    }

    /// Borrow the raw command line.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for QueueEntry {
    type Err = QueueEntryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<&str> for QueueEntry {
    type Error = QueueEntryError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
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
    /// Wrap an editor name, applying the `nvim` default: `None` or an empty
    /// name yields `nvim`; anything else is stored as given.
    pub fn new(name: Option<&str>) -> Self {
        match name.filter(|s| !s.is_empty()) {
            Some(s) => Self(s.to_owned()),
            None => Self("nvim".to_owned()),
        }
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
/// `Empty` is the canonical error for the deferred strict constructor path
/// (trim + blank reject, a P5/P6 behaviour change). It is declared as the
/// domain's public error type but **not produced** on the P5 path: the
/// lenient constructor stores every line verbatim and always returns `Ok`.
/// The `#[cfg(test)]` module below keeps the variant referenced.
#[derive(Debug, thiserror::Error)]
pub enum QueueEntryError {
    /// The command line was empty or whitespace-only.
    ///
    /// Reserved for the deferred strict `QueueEntry` constructor; not
    /// produced on the P5 (lenient) path.
    // Deferred landing point (strict-Empty reject is a P5/P6 change per PLAN.md); not constructed before then, hence the non-test dead_code suppression.
    #[cfg_attr(not(test), allow(dead_code))]
    #[error("command line is empty")]
    Empty,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_accepts_command_line() {
        let entry = QueueEntry::from_str("nix build .").unwrap();
        assert_eq!(entry.as_str(), "nix build .");
    }

    #[test]
    fn from_str_accepts_whitespace_verbatim() {
        // Lenient by design (behaviour preservation): blank is a valid
        // entry, stored verbatim. The strict `Empty` reject is deferred.
        let entry = QueueEntry::from_str("   ").unwrap();
        assert_eq!(entry.as_str(), "   ");
    }

    #[test]
    fn from_str_accepts_empty_verbatim() {
        let entry = QueueEntry::from_str("").unwrap();
        assert_eq!(entry.as_str(), "");
    }

    #[test]
    fn new_is_canonical_lenient_constructor() {
        let entry = QueueEntry::new("nix build .").unwrap();
        assert_eq!(entry.as_str(), "nix build .");

        let blank = QueueEntry::new("   ").unwrap();
        assert_eq!(blank.as_str(), "   ");
    }

    #[test]
    fn try_from_delegates_to_from_str() {
        let entry: QueueEntry = "nix build .".try_into().unwrap();
        assert_eq!(entry.as_str(), "nix build .");

        let blank: QueueEntry = "   ".try_into().unwrap();
        assert_eq!(blank.as_str(), "   ");
    }

    #[test]
    fn display_prints_raw_line() {
        let entry = QueueEntry::new("nix build .").unwrap();
        assert_eq!(entry.to_string(), "nix build .");
    }

    #[test]
    fn queue_path_keeps_path() {
        let path = QueuePath::new(PathBuf::from("/tmp/home/.config/encode_queue/commands.txt"));
        assert_eq!(
            path.as_path(),
            Path::new("/tmp/home/.config/encode_queue/commands.txt")
        );
        assert_eq!(
            path.to_string(),
            "/tmp/home/.config/encode_queue/commands.txt"
        );
    }

    #[test]
    fn editor_defaults_to_nvim_when_unset_or_empty() {
        assert_eq!(Editor::new(None).as_str(), "nvim");
        assert_eq!(Editor::new(Some("")).as_str(), "nvim");
        assert_eq!(Editor::new(Some("nvim")).as_str(), "nvim");
        assert_eq!(Editor::new(Some("code")).as_str(), "code");
        assert_eq!(Editor::new(Some("code")).to_string(), "code");
    }

    #[test]
    fn empty_error_variant_is_declared_for_deferred_strict_path() {
        // The lenient constructor never produces `Empty` on the P5 path;
        // this keeps the deferred-strict variant referenced so it cannot
        // trip a dead-code warning.
        let mut produced_empty = false;
        match QueueEntry::new("nix build .") {
            Ok(_) => {}
            Err(QueueEntryError::Empty) => {
                produced_empty = true;
            }
        }
        assert!(!produced_empty);
    }
}
