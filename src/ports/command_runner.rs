//! `CommandRunner` port: run a single queued command.
//!
//! Scaffold (WP01): the trait signature is final; `RunError` is a `thiserror`
//! skeleton (full variant set in WP03).

use crate::domain::queue::QueueEntry;

/// Errors surfaced at the command-runner boundary.
///
/// Skeleton; the concrete variants (`Spawn`, `Waited`, `Other`) land in WP03.
#[derive(Debug, thiserror::Error)]
pub enum RunError {}

/// Runs queued commands.
pub trait CommandRunner {
    /// Run a single entry, inheriting stdio.
    fn run(&self, entry: &QueueEntry) -> Result<(), RunError>;
}
