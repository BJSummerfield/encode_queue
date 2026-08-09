use std::io;
use thiserror::Error;

use super::command::{CommandText, InvalidCommandError};

#[derive(Error, Debug)]
pub enum QueueError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("HOME environment variable not set")]
    Env,
    #[error("queue is empty")]
    Empty,
    #[error("failed to spawn editor: {0}")]
    EditorSpawn(String),
    #[error(transparent)]
    InvalidCommand(#[from] InvalidCommandError),
}

pub trait QueueRepository {
    fn add(&mut self, command: &CommandText) -> Result<(), QueueError>;
    fn list(&self) -> Result<Vec<CommandText>, QueueError>;
    fn get_and_remove_first(&mut self) -> Result<Option<CommandText>, QueueError>;
    fn open_in_editor(&self) -> Result<(), QueueError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_converts_to_queue_error() {
        let io_err = io::Error::new(io::ErrorKind::Other, "test");
        let queue_err: QueueError = io_err.into();
        assert!(matches!(queue_err, QueueError::Io(_)));
    }

    #[test]
    fn invalid_command_converts_to_queue_error() {
        let cmd_err = InvalidCommandError::Empty;
        let queue_err: QueueError = cmd_err.into();
        assert!(matches!(queue_err, QueueError::InvalidCommand(_)));
    }
}