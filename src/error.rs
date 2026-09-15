//! Public (binary-edge) error type (C005/C032).
//!
//! Leaf module: depends only on `std` and `thiserror`, so ports, use cases
//! and adapters may all depend on it without an outward behavioral edge
//! (plan D1). Domain errors (`crate::domain::error::DomainError`) are mapped
//! onto this type at the config boundaries by explicit `map_err` (plan D2).

use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// Exit code in a public message: the numeric code, or the literal `None`
/// when the process was terminated by a signal (plan §3 behavior table).
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

/// The public error type for the binary edge.
///
/// Every variant carries a hand-written terse `#[error]` message (C032: the
/// CLI edge shows the terse message; the OS detail stays in `#[source]`
/// for logs, never printed raw).
#[derive(Debug, Error)]
pub enum CliError {
    /// `$HOME` is missing or not valid UTF-8.
    #[error("HOME environment variable is not set or not valid UTF-8")]
    ConfigHome,

    /// `$EDITOR` is set but not valid UTF-8.
    #[error("EDITOR environment variable is not valid UTF-8")]
    ConfigEditor,

    /// The queue file could not be read (e.g. missing file).
    #[error("could not read queue {path}")]
    QueueRead {
        /// The underlying OS error.
        #[source]
        source: io::Error,
        /// The queue file path.
        path: PathBuf,
    },

    /// The queue file could not be written (permissions, disk, …).
    #[error("could not write queue {path}")]
    QueueWrite {
        /// The underlying OS error.
        #[source]
        source: io::Error,
        /// The queue file path (or queue directory for `create_dir_all`).
        path: PathBuf,
    },

    /// `fish` could not be spawned for a queued command (spawn failure,
    /// distinct from a ran-but-non-zero exit — `CommandFailed`).
    #[error("could not launch fish")]
    SpawnFailed {
        /// The underlying OS error.
        #[source]
        source: io::Error,
    },

    /// A queued command ran and exited non-zero. `code` is `None` when the
    /// process was terminated by a signal.
    #[error("queued command failed (exit status: {code})")]
    CommandFailed {
        /// The process exit code, if any.
        code: ExitCode,
    },

    /// The editor could not be spawned.
    #[error("could not launch editor {editor:?}")]
    EditorSpawn {
        /// The underlying OS error.
        #[source]
        source: io::Error,
        /// The editor program name that failed to spawn.
        editor: String,
    },

    /// The editor ran but exited non-zero (intentional fix: a failing
    /// editor no longer reports success). `code` is `None` on signal.
    #[error("editor {editor:?} exited non-zero (exit status: {code})")]
    EditorExit {
        /// The editor program name.
        editor: String,
        /// The process exit code, if any.
        code: ExitCode,
    },

    /// A required argument was somehow missing (defensive; `required(true)`
    /// makes this unreachable after a successful parse — kept from the CT002
    /// seam because removing it would require a panic (C004)).
    #[error("missing required argument: COMMAND")]
    MissingCommand,
}
