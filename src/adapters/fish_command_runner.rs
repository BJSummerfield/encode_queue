use std::process::{Command as SystemCommand, Stdio};

use crate::domain::entry::QueueEntry;
use crate::error::CliError;
use crate::ports::command_runner::{CommandExit, CommandRunner};

pub struct FishCommandRunner;

impl CommandRunner for FishCommandRunner {
    fn run(&self, entry: &QueueEntry) -> Result<CommandExit, CliError> {
        let status = SystemCommand::new("fish")
            .arg("-c")
            .arg(entry.as_str().trim())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|source| CliError::SpawnFailed { source })?;
        Ok(CommandExit::new(status))
    }
}
