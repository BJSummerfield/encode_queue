
use std::ffi::OsString;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("queue entry is empty: {0:?}")]
    EmptyQueueEntry(String),

    #[error("HOME environment variable is not set")]
    HomeMissing,

    #[error("HOME environment variable is not valid UTF-8: {0:?}")]
    HomeNotUtf8(OsString),

    #[error("queue path is not valid UTF-8: {0:?}")]
    QueuePathNotUtf8(OsString),

    #[error("EDITOR environment variable is not valid UTF-8: {0:?}")]
    EditorNotUtf8(OsString),
}
