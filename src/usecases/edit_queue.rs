use std::env;
use std::io;
use std::path::PathBuf;
use std::process::Command as SystemCommand;

/// Opens the queue file in the user's editor (`$EDITOR`, default `nvim`).
///
/// No port by design: exactly two ports are permitted (spec) and the editor
/// is a one-shot side effect on the config file, not the queue store or a
/// queued command. CT002 types the path; CT003 replaces the `expect`.
pub struct EditQueue {
    queue_path: PathBuf,
}

impl EditQueue {
    pub fn new(queue_path: PathBuf) -> Self {
        Self { queue_path }
    }

    pub fn execute(&self) -> io::Result<()> {
        let editor = env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string());

        SystemCommand::new(editor)
            .arg(self.queue_path.to_str().unwrap())
            .status()
            .expect("Failed to open editor");

        Ok(())
    }
}
