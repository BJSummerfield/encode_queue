
use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitCode(Option<i32>);

impl From<Option<i32>> for ExitCode {
    fn from(code: Option<i32>) -> Self {
        Self(code)
    }
}

impl std::fmt::Display for ExitCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(code) => write!(f, "{code}"),
            None => f.write_str("None"),
        }
    }
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error("HOME environment variable is not set or not valid UTF-8")]
    ConfigHome,

    #[error("EDITOR environment variable is not valid UTF-8")]
    ConfigEditor,

    #[error("could not read queue {path}")]
    QueueRead {
        #[source]
        source: io::Error,
        path: PathBuf,
    },

    #[error("could not write queue {path}")]
    QueueWrite {
        #[source]
        source: io::Error,
        path: PathBuf,
    },

    #[error("could not launch fish")]
    SpawnFailed {
        #[source]
        source: io::Error,
    },

    #[error("queued command failed (exit status: {code})")]
    CommandFailed {
        code: ExitCode,
    },

    #[error("could not launch editor {editor:?}")]
    EditorSpawn {
        #[source]
        source: io::Error,
        editor: String,
    },

    #[error("editor {editor:?} exited non-zero (exit status: {code})")]
    EditorExit {
        editor: String,
        code: ExitCode,
    },

    #[error("missing required argument: COMMAND")]
    MissingCommand,
}
