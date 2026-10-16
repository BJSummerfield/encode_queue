//! Domain record of a queued command that exited non-zero (P6.F3).
//!
//! `FailureRecord` is a boundary record, not a fallible domain entity: its
//! fields are populated by the service from values the runner already
//! validated (the exit code) or from domain newtypes (`QueueEntry`), so no
//! constructor is needed. The `QueueStore` writes it to its failure log in
//! the queue directory.

use crate::domain::queue::QueueEntry;

/// A record of a queued command that exited non-zero.
///
/// Written by the queue store to its failure log (the log file the store
/// keeps in the queue directory) as one
/// `<unix_ts>\t<exit_code>\t<command>\n` line. Public fields are deliberate:
/// the record is a flat boundary value populated from already-validated
/// inputs, not a domain entity with invariants to protect (OP5 targets
/// fallible newtypes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureRecord {
    /// Unix timestamp (seconds) at which the failure was observed.
    pub timestamp: u64,
    /// The command's exit code.
    pub exit_code: i32,
    /// The command that failed — the trimmed entry the runner executed.
    pub command: QueueEntry,
}
