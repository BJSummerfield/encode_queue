use std::process::ExitStatus;

use crate::domain::entry::QueueEntry;
use crate::error::CliError;

/// Outcome of a command that actually ran. Spawn failure is
/// `Err(CliError::SpawnFailed)`; a ran-but-non-zero exit is an
/// `Ok(CommandExit)` the caller checks via `success()`.
pub struct CommandExit {
    status: ExitStatus,
}

impl CommandExit {
    pub fn new(status: ExitStatus) -> Self {
        Self { status }
    }

    /// The process exit code, if the process exited normally.
    pub fn code(&self) -> Option<i32> {
        self.status.code()
    }

    /// True when the process exited with status 0.
    pub fn success(&self) -> bool {
        self.status.success()
    }
}

/// Outbound port: runs one queued command.
pub trait CommandRunner {
    fn run(&self, entry: &QueueEntry) -> Result<CommandExit, CliError>;
}
