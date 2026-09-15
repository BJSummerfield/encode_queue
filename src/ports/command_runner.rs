use std::process::ExitStatus;

use crate::domain::entry::QueueEntry;
use crate::error::CliError;

pub struct CommandExit {
    status: ExitStatus,
}

impl CommandExit {
    pub fn new(status: ExitStatus) -> Self {
        Self { status }
    }

    pub fn code(&self) -> Option<i32> {
        self.status.code()
    }

    pub fn success(&self) -> bool {
        self.status.success()
    }
}

pub trait CommandRunner {
    fn run(&self, entry: &QueueEntry) -> Result<CommandExit, CliError>;
}
