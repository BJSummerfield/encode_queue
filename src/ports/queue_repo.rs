use crate::domain::entry::QueueEntry;
use crate::error::CliError;

/// Outbound port: the queue's persistent store.
pub trait QueueRepo {
    /// Read all queue lines in order (missing file is a `CliError::QueueRead`
    /// carrying the path; the raw OS error is its `#[source]`).
    fn read_entries(&self) -> Result<Vec<QueueEntry>, CliError>;

    /// Append one entry as a new line (creates the file if absent, as today).
    fn append_entry(&self, entry: &QueueEntry) -> Result<(), CliError>;

    /// Replace the whole queue with `remaining` (truncate + rewrite).
    fn replace_with_remaining(&self, remaining: &[QueueEntry]) -> Result<(), CliError>;
}
