use crate::domain::{CommandText, QueueError, QueueRepository};
use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    process::Command,
};

pub struct FileQueue {
    path: PathBuf,
}

impl FileQueue {
    pub fn new() -> Result<Self, QueueError> {
        let home = env::var("HOME").map_err(|_| QueueError::Env)?;
        Self::with_dir(PathBuf::from(home))
    }

    pub fn with_dir(home: PathBuf) -> Result<Self, QueueError> {
        let dir = home.join(".config/encode_queue");
        fs::create_dir_all(&dir).map_err(QueueError::Io)?;
        Ok(Self {
            path: dir.join("commands.txt"),
        })
    }

    pub fn with_path(path: PathBuf) -> Self {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        Self { path }
    }
}

impl QueueRepository for FileQueue {
    fn add(&mut self, command: &CommandText) -> Result<(), QueueError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{}", command.as_str())?;
        Ok(())
    }

    fn list(&self) -> Result<Vec<CommandText>, QueueError> {
        let contents = fs::read_to_string(&self.path).unwrap_or_default();
        let mut commands = Vec::new();
        for line in contents.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                commands.push(CommandText::new(trimmed)?);
            }
        }
        Ok(commands)
    }

    fn get_and_remove_first(&mut self) -> Result<Option<CommandText>, QueueError> {
        let contents = fs::read_to_string(&self.path).unwrap_or_default();
        let mut lines: Vec<String> = contents.lines().map(|l| l.to_string()).collect();

        // Skip leading empty lines
        while let Some(line) = lines.first() {
            if line.trim().is_empty() {
                lines.remove(0);
            } else {
                break;
            }
        }

        if lines.is_empty() {
            return Ok(None);
        }

        let first = lines.remove(0);
        let command = CommandText::new(&first)?;

        let output = if lines.is_empty() {
            String::new()
        } else {
            lines.join("\n") + "\n"
        };
        fs::write(&self.path, output)?;

        Ok(Some(command))
    }

    fn open_in_editor(&self) -> Result<(), QueueError> {
        let editor = env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string());
        let status = Command::new(editor)
            .arg(&self.path)
            .status()
            .map_err(|e| QueueError::EditorSpawn(e.to_string()))?;

        if !status.success() {
            return Err(QueueError::EditorSpawn(format!(
                "editor exited with status {}",
                status
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn test_queue() -> (FileQueue, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("commands.txt");
        (FileQueue::with_path(path), dir)
    }

    #[test]
    fn add_and_list() {
        let (mut queue, _dir) = test_queue();
        let cmd = CommandText::new("echo hello").unwrap();
        queue.add(&cmd).unwrap();

        let commands = queue.list().unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].as_str(), "echo hello");
    }

    #[test]
    fn add_multiple_and_list() {
        let (mut queue, _dir) = test_queue();
        queue.add(&CommandText::new("cmd one").unwrap()).unwrap();
        queue.add(&CommandText::new("cmd two").unwrap()).unwrap();

        let commands = queue.list().unwrap();
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].as_str(), "cmd one");
        assert_eq!(commands[1].as_str(), "cmd two");
    }

    #[test]
    fn get_and_remove_first_fifo() {
        let (mut queue, _dir) = test_queue();
        queue.add(&CommandText::new("first").unwrap()).unwrap();
        queue.add(&CommandText::new("second").unwrap()).unwrap();

        let first = queue.get_and_remove_first().unwrap().unwrap();
        assert_eq!(first.as_str(), "first");

        let remaining = queue.list().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].as_str(), "second");
    }

    #[test]
    fn get_and_remove_first_empty_returns_none() {
        let (mut queue, _dir) = test_queue();
        let result = queue.get_and_remove_first().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_empty_queue_returns_empty_vec() {
        let (queue, _dir) = test_queue();
        let commands = queue.list().unwrap();
        assert!(commands.is_empty());
    }

    #[test]
    fn list_filters_empty_lines() {
        let (mut queue, _dir) = test_queue();
        queue.add(&CommandText::new("real cmd").unwrap()).unwrap();

        // Manually insert empty lines
        fs::write(&queue.path, "\nreal cmd\n\n").unwrap();

        let commands = queue.list().unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].as_str(), "real cmd");
    }

    #[test]
    fn get_and_remove_first_skips_leading_empty_lines() {
        let (mut queue, _dir) = test_queue();
        queue.add(&CommandText::new("cmd").unwrap()).unwrap();

        // Prepend empty lines
        fs::write(&queue.path, "\n\ncmd\n").unwrap();

        let result = queue.get_and_remove_first().unwrap().unwrap();
        assert_eq!(result.as_str(), "cmd");
    }

    #[test]
    fn with_path_creates_parent_directory() {
        let dir = TempDir::new().unwrap();
        let nested_path = dir.path().join("nested/deep/commands.txt");
        let mut queue = FileQueue::with_path(nested_path.clone());

        assert!(nested_path.exists() || nested_path.parent().unwrap().exists());
        let cmd = CommandText::new("test").unwrap();
        queue.add(&cmd).unwrap();
    }

    #[test]
    fn get_and_remove_first_preserves_trailing_newline() {
        let (mut queue, _dir) = test_queue();
        queue.add(&CommandText::new("one").unwrap()).unwrap();
        queue.add(&CommandText::new("two").unwrap()).unwrap();
        queue.add(&CommandText::new("three").unwrap()).unwrap();

        let first = queue.get_and_remove_first().unwrap().unwrap();
        assert_eq!(first.as_str(), "one");

        // Verify remaining commands are intact
        let remaining = queue.list().unwrap();
        assert_eq!(remaining.len(), 2);
        assert_eq!(remaining[0].as_str(), "two");
        assert_eq!(remaining[1].as_str(), "three");
    }
}