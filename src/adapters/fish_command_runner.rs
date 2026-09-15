use std::process::{Command as SystemCommand, Stdio};

use crate::domain::entry::QueueEntry;
use crate::ports::command_runner::{CommandExit, CommandRunner};

/// Runs a queued line as `fish -c <line>` with inherited stdio.
pub struct FishCommandRunner;

impl CommandRunner for FishCommandRunner {
    fn run(&self, entry: &QueueEntry) -> std::io::Result<CommandExit> {
        let status = SystemCommand::new("fish")
            .arg("-c")
            .arg(entry.as_str().trim())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;
        Ok(CommandExit::new(status))
    }
}
