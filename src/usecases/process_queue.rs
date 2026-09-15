use crate::error::CliError;
use crate::ports::command_runner::CommandRunner;
use crate::ports::queue_repo::QueueRepo;

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
                None => break,
            };
            if !head.as_str().trim().is_empty() {
                let exit = self.runner.run(head)?;
                if !exit.success() {
                    return Err(CliError::CommandFailed { code: exit.code().into() });
                }
            }
            self.repo.replace_with_remaining(&entries[1..])?;
        }
        Ok(())
    }
}
