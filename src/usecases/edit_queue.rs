use std::env;
use std::ffi::OsString;
use std::process::Command as SystemCommand;

use crate::domain::config::{Editor, QueuePath};

pub struct EditQueue {
    queue_path: QueuePath,
}

impl EditQueue {
    pub fn new(queue_path: QueuePath) -> Self {
        Self { queue_path }
    }

    pub fn execute(&self) -> Result<(), crate::error::CliError> {
        let editor_os = env::var_os("EDITOR").unwrap_or_else(|| OsString::from("nvim"));
        let editor = Editor::try_from(editor_os)
            .map_err(|_| crate::error::CliError::ConfigEditor)?;

        let status = SystemCommand::new(editor.as_str())
            .arg(self.queue_path.as_str())
            .status()
            .map_err(|source| crate::error::CliError::EditorSpawn {
                source,
                editor: editor.as_str().to_string(),
            })?;

        if !status.success() {
            return Err(crate::error::CliError::EditorExit {
                editor: editor.as_str().to_string(),
                code: status.code().into(),
            });
        }

        Ok(())
    }
}
