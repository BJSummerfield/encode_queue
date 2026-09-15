//! Domain-level conversion/configuration errors (CT002).
//!
//! These are *domain* errors, distinct from the public `CliError` (CT003,
//! C032). The `main()` boundary maps them onto its concrete error type.

use std::ffi::OsString;
use thiserror::Error;

/// Errors raised while converting boundary values (env vars, CLI args, file
/// lines) into typed domain values.
#[derive(Debug, Error)]
pub enum DomainError {
    /// A queue entry failed validation (empty/whitespace-only).
    #[error("queue entry is empty: {0:?}")]
    EmptyQueueEntry(String),

    /// `$HOME` is not set.
    #[error("HOME environment variable is not set")]
    HomeMissing,

    /// `$HOME` is set but not valid UTF-8.
    #[error("HOME environment variable is not valid UTF-8: {0:?}")]
    HomeNotUtf8(OsString),

    /// The assembled queue path is not valid UTF-8 (defensive; a UTF-8 `$HOME`
    /// joined with a UTF-8 literal is always UTF-8).
    #[error("queue path is not valid UTF-8: {0:?}")]
    QueuePathNotUtf8(OsString),

    /// `$EDITOR` is not valid UTF-8.
    #[error("EDITOR environment variable is not valid UTF-8: {0:?}")]
    EditorNotUtf8(OsString),
}
