use crate::domain::entry::QueueEntry;
use crate::error::CliError;

pub trait QueueRepo {
    fn read_entries(&self) -> Result<Vec<QueueEntry>, CliError>;

    fn append_entry(&self, entry: &QueueEntry) -> Result<(), CliError>;

    fn replace_with_remaining(&self, remaining: &[QueueEntry]) -> Result<(), CliError>;
}
