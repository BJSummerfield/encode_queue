use std::io;

use crate::ports::command_runner::CommandRunner;
use crate::ports::queue_repo::QueueRepo;

/// Drains the queue: execute the first line, drop it, repeat until empty.
pub struct ProcessQueue<R: QueueRepo, C: CommandRunner> {
    repo: R,
    runner: C,
}

impl<R: QueueRepo, C: CommandRunner> ProcessQueue<R, C> {
    pub fn new(repo: R, runner: C) -> Self {
        Self { repo, runner }
    }

    pub fn execute(&self) -> io::Result<()> {
        loop {
            let entries = self.repo.read_entries()?;
            let head = match entries.first() {
                Some(head) => head,
                None => break, // read_line == 0 in the current code
            };
            // CT002: guaranteed true now (read edge filters blank lines);
            // retained to preserve the original drain shape.
            if !head.as_str().trim().is_empty() {
                // Blank lines are skipped without spawning, as today.
                let _ = self.runner.run(head)?; // exit status ignored, as today
            }
            self.repo.replace_with_remaining(&entries[1..])?;
        }
        Ok(())
    }
}
