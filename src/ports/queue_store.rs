//! `QueueStore` port: read, append, and rewrite the queue file.
//!
//! Scaffold (WP01): the trait signature is final; `QueueStoreError` is a
//! `thiserror` skeleton (full variant set in WP03). The store owns its
//! `QueuePath`, so no raw path crosses the port (ARCH-03).

use crate::domain::queue::QueueEntry;

/// Errors surfaced at the queue-store boundary.
///
/// Skeleton; the concrete variants (`QueueFileMissing`, `Io`) land in WP03.
#[derive(Debug, thiserror::Error)]
pub enum QueueStoreError {}

/// A store of queue entries, owned by the queue file in `outbound/`.
pub trait QueueStore {
    /// Read every entry in the queue, in order.
    fn read_all(&self) -> Result<Vec<QueueEntry>, QueueStoreError>;

    /// Append a single entry to the queue.
    fn append(&self, entry: &QueueEntry) -> Result<(), QueueStoreError>;

    /// Remove the first entry and rewrite the queue without it.
    fn drop_first(&self) -> Result<(), QueueStoreError>;
}
