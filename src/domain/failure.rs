//! Domain record of a queued command that exited non-zero (P6.F3).
//!
//! `FailureRecord` is a flat boundary record, not a fallible domain entity:
//! its fields are populated by the service from values the runner already
//! validated (the exit code) or from domain newtypes (`QueueEntry`). `new`
//! is its only entry point, and the fields are read through accessors,
//! never a `pub` field (OP5, TY-03). The `QueueStore` writes it to its
//! failure log in the queue directory.

use crate::domain::queue::QueueEntry;

/// A record of a queued command that exited non-zero.
///
/// Written by the queue store to its failure log (the log file the store
/// keeps in the queue directory) as one
/// `<unix_ts>\t<exit_code>\t<command>\n` line. The record is a flat boundary
/// value, not a newtype: it is serialized as a tab-joined line, so its
/// fields carry no invariants to protect. The constructor is the only entry
/// point, and the fields are read through accessors (OP5, TY-03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureRecord {
    /// Unix timestamp (seconds) at which the failure was observed.
    timestamp: u64,
    /// The command's exit code.
    exit_code: i32,
    /// The command that failed — the trimmed entry the runner executed.
    command: QueueEntry,
}

impl FailureRecord {
    /// Record an already-validated failure (the only entry point; OP5,
    /// TY-03).
    pub fn new(timestamp: u64, exit_code: i32, command: QueueEntry) -> Self {
        Self {
            timestamp,
            exit_code,
            command,
        }
    }

    /// Unix timestamp (seconds) at which the failure was observed.
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// The command's exit code.
    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }

    /// The command that failed — the trimmed entry the runner executed.
    pub fn command(&self) -> &QueueEntry {
        &self.command
    }
}
