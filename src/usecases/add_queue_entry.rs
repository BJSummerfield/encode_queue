use std::io;

use crate::domain::entry::QueueEntry;
use crate::ports::queue_repo::QueueRepo;

/// Adds one command to the queue.
pub struct AddQueueEntry<R: QueueRepo> {
    repo: R,
}

impl<R: QueueRepo> AddQueueEntry<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub fn execute(&self, entry: &QueueEntry) -> io::Result<()> {
        self.repo.append_entry(entry)
    }
}
