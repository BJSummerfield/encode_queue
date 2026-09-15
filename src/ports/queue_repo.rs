use std::io;

use crate::domain::entry::QueueEntry;

/// Outbound port: the queue's persistent store.
pub trait QueueRepo {
    /// Read all queue lines in order (missing file is an `io::Error`, as today).
    fn read_entries(&self) -> io::Result<Vec<QueueEntry>>;

    /// Append one entry as a new line (creates the file if absent, as today).
    fn append_entry(&self, entry: &QueueEntry) -> io::Result<()>;

    /// Replace the whole queue with `remaining` (truncate + rewrite).
    fn replace_with_remaining(&self, remaining: &[QueueEntry]) -> io::Result<()>;
}
