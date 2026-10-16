//! `QueueStore` port: read, append, and rewrite the queue file, plus
//! failure logging (`append_failure` → the store's failure log, P6.F3).
//!
//! The trait signature is final: it takes only domain newtypes and returns
//! only `QueueStoreError` (ARCH-03). The store owns its `QueuePath`, so no
//! raw path crosses the port.

use std::path::PathBuf;

use crate::domain::failure::FailureRecord;
use crate::domain::queue::QueueEntry;

/// Errors surfaced at the queue-store boundary.
///
/// One enum beside the port (ERR-02); every expected failure is a named
/// variant with an owned payload plus the `Io` catch-all (ERR-04, GEN-03).
#[derive(Debug, thiserror::Error)]
pub enum QueueStoreError {
    /// The queue file does not exist.
    #[error("queue file not found at {0}")]
    QueueFileMissing(PathBuf),
    /// Any other file I/O failure the adapter could not classify.
    #[error("queue file I/O failed: {0}")]
    Io(#[source] std::io::Error),
}

/// A store of queue entries, owned by the queue file in `outbound/`.
pub trait QueueStore {
    /// Read every entry in the queue, in order.
    fn read_all(&self) -> Result<Vec<QueueEntry>, QueueStoreError>;

    /// Append a single entry to the queue.
    fn append(&self, entry: &QueueEntry) -> Result<(), QueueStoreError>;

    /// Remove the first entry and rewrite the queue without it.
    fn drop_first(&self) -> Result<(), QueueStoreError>;

    /// Replace the entire queue with `entries` (truncate + rewrite).
    fn write_all(&self, entries: &[QueueEntry]) -> Result<(), QueueStoreError>;

    /// Append one failure record to the store's failure log (P6.F3).
    ///
    /// Called by the service when a queued command exits non-zero,
    /// before the failed entry is removed from the queue. I/O failures map
    /// to the existing `Io` catch-all (no new error variant, ERR-04).
    fn append_failure(&self, record: &FailureRecord) -> Result<(), QueueStoreError>;
}

#[cfg(test)]
mod tests {
    use super::QueueStoreError;
    use std::path::PathBuf;

    #[test]
    fn variants_carry_owned_payloads() {
        assert!(matches!(
            QueueStoreError::QueueFileMissing(PathBuf::from("/tmp/commands.txt")),
            QueueStoreError::QueueFileMissing(_)
        ));
        assert!(matches!(
            QueueStoreError::Io(std::io::Error::other("test I/O failure")),
            QueueStoreError::Io(_)
        ));
    }
}
