use crate::error::CliError;
use crate::ports::command_runner::CommandRunner;
use crate::ports::queue_repo::QueueRepo;

/// Drains the queue: execute the first line, drop it, repeat until empty.
/// A ran-but-non-zero command fails with `CliError::CommandFailed` and its
/// line is NOT consumed (plan D6).
pub struct ProcessQueue<R: QueueRepo, C: CommandRunner> {
    repo: R,
    runner: C,
}

impl<R: QueueRepo, C: CommandRunner> ProcessQueue<R, C> {
    pub fn new(repo: R, runner: C) -> Self {
        Self { repo, runner }
    }

    pub fn execute(&self) -> Result<(), CliError> {
        loop {
            let entries = self.repo.read_entries()?;
            let head = match entries.first() {
                Some(head) => head,
                None => break, // empty queue — done
            };
            // CT002: guaranteed true (read edge filters blank lines); retained
            // to preserve the drain shape.
            if !head.as_str().trim().is_empty() {
                let exit = self.runner.run(head)?; // spawn failure: Err -> not consumed
                if !exit.success() {
                    // Ran but non-zero (plan D6): fail with the exit status and
                    // do NOT consume the line — a failed command stays at the
                    // head for inspection/retry.
                    return Err(CliError::CommandFailed { code: exit.code().into() });
                }
            }
            self.repo.replace_with_remaining(&entries[1..])?;
        }
        Ok(())
    }
}
