use std::env;
use std::ffi::OsString;
use std::process::Command as SystemCommand;

use crate::domain::config::{Editor, QueuePath};

/// Opens the queue file in the user's editor (`$EDITOR`, default `nvim`).
///
/// No port by design (unchanged from CT001). The path and the editor program
/// name are typed via `TryFrom` at the boundary (CT002); spawn and exit
/// failures map to `CliError::EditorSpawn` / `CliError::EditorExit` (CT003,
/// plan D5).
pub struct EditQueue {
    queue_path: QueuePath,
}

impl EditQueue {
    pub fn new(queue_path: QueuePath) -> Self {
        Self { queue_path }
    }

    pub fn execute(&self) -> Result<(), crate::error::CliError> {
        // `Option::unwrap_or_else` is total (non-panicking) — allowed (C004).
        let editor_os = env::var_os("EDITOR").unwrap_or_else(|| OsString::from("nvim"));
        // Only reachable domain error here is EditorNotUtf8 -> public
        // ConfigEditor (plan D2).
        let editor = Editor::try_from(editor_os)
            .map_err(|_| crate::error::CliError::ConfigEditor)?;

        let status = SystemCommand::new(editor.as_str())
            .arg(self.queue_path.as_str()) // typed path — no to_str().unwrap()
            .status()
            .map_err(|source| crate::error::CliError::EditorSpawn {
                source,
                editor: editor.as_str().to_string(),
            })?;

        // Intentional behavior change (plan D5): a non-zero editor exit is a
        // failure (was: silently reported as success).
        if !status.success() {
            return Err(crate::error::CliError::EditorExit {
                editor: editor.as_str().to_string(),
                code: status.code().into(),
            });
        }

        Ok(())
    }
}
