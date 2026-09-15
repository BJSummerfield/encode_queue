use std::io;

use crate::ports::queue_repo::QueueRepo;

/// Lists the queue: one entry per printed line.
pub struct ListQueue<R: QueueRepo> {
    repo: R,
}

impl<R: QueueRepo> ListQueue<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub fn execute(&self) -> io::Result<()> {
        for entry in self.repo.read_entries()? {
            println!("{}", entry.as_str());
        }
        Ok(())
    }
}
