use std::env;
use std::ffi::OsString;
use std::process::Command as SystemCommand;

use crate::domain::config::{Editor, QueuePath};
use crate::domain::error::DomainError;

/// Opens the queue file in the user's editor (`$EDITOR`, default `nvim`).
///
/// No port by design (unchanged from CT001). CT002 types the path and the
/// editor program name via `TryFrom`; the `.status().expect(...)` panic is
/// left for CT003 (public CliError + exit code).
pub struct EditQueue {
    queue_path: QueuePath,
}

impl EditQueue {
    pub fn new(queue_path: QueuePath) -> Self {
        Self { queue_path }
    }

    /// A non-UTF-8 `$EDITOR` is a domain error here (was a silent `nvim`
    /// fallback). The editor *spawn* error is still an `expect` panic in this
    /// intermediate state — CT003 replaces it.
    pub fn execute(&self) -> Result<(), DomainError> {
        // `Option::unwrap_or_else` is total (non-panicking) — allowed (C004).
        let editor_os = env::var_os("EDITOR").unwrap_or_else(|| OsString::from("nvim"));
        let editor = Editor::try_from(editor_os)?;

        SystemCommand::new(editor.as_str())
            .arg(self.queue_path.as_str()) // typed path — no to_str().unwrap()
            .status()
            .expect("Failed to open editor"); // CT003

        Ok(())
    }
}
