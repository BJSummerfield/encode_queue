use std::io;
use std::process::ExitStatus;

use crate::domain::entry::QueueEntry;

/// Outcome of a command that actually ran. Spawn failure is `Err(io::Error)`.
pub struct CommandExit {
    status: ExitStatus,
}

// Exit status is not consumed until CT003 wires it into error handling;
// the API surface is part of the CT001 plan and must stay.
#[allow(dead_code)]
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

/// Outbound port: runs one queued command.
pub trait CommandRunner {
    fn run(&self, entry: &QueueEntry) -> io::Result<CommandExit>;
}
