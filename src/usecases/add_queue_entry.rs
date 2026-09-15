use crate::domain::entry::QueueEntry;
use crate::error::CliError;
use crate::ports::queue_repo::QueueRepo;

pub struct AddQueueEntry<R: QueueRepo> {
    repo: R,
}

impl<R: QueueRepo> AddQueueEntry<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub fn execute(&self, entry: &QueueEntry) -> Result<(), CliError> {
        self.repo.append_entry(entry)
    }
}
