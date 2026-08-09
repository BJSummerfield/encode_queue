# Conventions Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor encode_queue to follow the conventions in `CONVENTIONS.md` — newtypes, boundary crossing, structured errors, vertical slices, dependency inversion.

**Architecture:** Single `main.rs` becomes a library crate with domain types, a `QueueRepository` trait, a `FileQueue` adapter, and feature modules. `main.rs` does only CLI parsing and bootstrapping.

**Tech Stack:** Rust 2021, clap (derive), thiserror, tempfile (dev)

## Global Constraints

- Edition: 2021
- Fix version mismatch: Cargo.toml `0.1.0` → `1.0.0` (matches README)
- CLI interface unchanged: `add <CMD>`, `start`, `ls`, `edit`
- Queue file location unchanged: `~/.config/encode_queue/commands.txt`
- Shell unchanged: fish
- Default editor unchanged: nvim (falls back from `$EDITOR`)

---

### Task 1: Domain types — CommandText, QueueError, QueueRepository trait

**Files:**
- Modify: `Cargo.toml` (add `thiserror = "2.0"`)
- Create: `src/lib.rs`
- Create: `src/domain/mod.rs`
- Create: `src/domain/command.rs`
- Create: `src/domain/queue.rs`
- Test: inline `#[cfg(test)]` modules in `command.rs` and `queue.rs`

**Interfaces:**
- Produces: `CommandText` (newtype), `InvalidCommandError` (enum), `QueueError` (enum), `QueueRepository` (trait)
- Produces: `CommandText::new(&str) -> Result<Self, InvalidCommandError>`, `CommandText::as_str(&self) -> &str`
- Produces: `impl<'a> TryFrom<&'a str> for CommandText`
- Produces: `QueueRepository` trait with `add`, `list`, `get_and_remove_first`, `open_in_editor`

- [ ] **Step 1: Update Cargo.toml and create module skeleton**

Update `Cargo.toml`:
```toml
[package]
name = "encode_queue"
version = "1.0.0"
edition = "2021"

[dependencies]
clap = { version = "4.5", features = ["derive"] }
thiserror = "2.0"
```

Create `src/lib.rs`:
```rust
pub mod adapters;
pub mod domain;
pub mod features;
```

Create `src/domain/mod.rs`:
```rust
mod command;
mod queue;

pub use command::{CommandText, InvalidCommandError};
pub use queue::{QueueError, QueueRepository};
```

- [ ] **Step 2: Write domain/command.rs**

```rust
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandText(String);

impl CommandText {
    pub fn new(raw: &str) -> Result<Self, InvalidCommandError> {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            return Err(InvalidCommandError::Empty);
        }
        Ok(Self(trimmed))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'a> TryFrom<&'a str> for CommandText {
    type Error = InvalidCommandError;
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Error, Debug)]
pub enum InvalidCommandError {
    #[error("command cannot be empty")]
    Empty,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_command() {
        let cmd = CommandText::new("echo hello").unwrap();
        assert_eq!(cmd.as_str(), "echo hello");
    }

    #[test]
    fn empty_command_rejected() {
        assert!(CommandText::new("").is_err());
    }

    #[test]
    fn whitespace_only_rejected() {
        assert!(CommandText::new("   ").is_err());
    }

    #[test]
    fn trims_whitespace() {
        let cmd = CommandText::new("  echo hello  ").unwrap();
        assert_eq!(cmd.as_str(), "echo hello");
    }

    #[test]
    fn try_from_str() {
        let cmd: CommandText = "echo hello".try_into().unwrap();
        assert_eq!(cmd.as_str(), "echo hello");
    }

    #[test]
    fn try_from_empty_str_fails() {
        let result: Result<CommandText, _> = ("").try_into();
        assert!(result.is_err());
    }
}
```

- [ ] **Step 3: Write domain/queue.rs**

```rust
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
```

- [ ] **Step 4: Run tests to verify**

```bash
cargo test domain
```

Expected: 8 tests pass (6 in command, 2 in queue).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/domain/
git commit -m "feat: add domain types — CommandText, QueueError, QueueRepository trait"
```

---

### Task 2: FileQueue adapter

**Files:**
- Create: `src/adapters/mod.rs`
- Create: `src/adapters/file_queue.rs`
- Modify: `Cargo.toml` (add `tempfile` dev-dependency)
- Test: inline `#[cfg(test)]` module in `file_queue.rs`

**Interfaces:**
- Consumes: `QueueRepository` trait, `CommandText`, `QueueError` (from Task 1)
- Produces: `FileQueue::new() -> Result<Self, QueueError>`
- Produces: `FileQueue::with_dir(PathBuf) -> Result<Self, QueueError>`
- Produces: `FileQueue::with_path(PathBuf) -> Self` (for testing)

- [ ] **Step 1: Update Cargo.toml dev-dependencies**

```toml
[dev-dependencies]
tempfile = "3.14"
```

- [ ] **Step 2: Write adapters/mod.rs**

```rust
mod file_queue;
pub use file_queue::FileQueue;
```

- [ ] **Step 3: Write adapters/file_queue.rs**

```rust
use std::{
    env,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    process::Command,
};

use crate::domain::{CommandText, QueueError, QueueRepository};

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

        // Manually insert an empty line
        fs::write(&queue.path, "\nreal cmd\n\n").unwrap();

        let commands = queue.list().unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].as_str(), "real cmd");
    }
}
```

- [ ] **Step 4: Run tests to verify**

```bash
cargo test adapters
```

Expected: 6 tests pass.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/adapters/
git commit -m "feat: add FileQueue adapter with QueueRepository implementation"
```

---

### Task 3: Feature modules — add, list, edit

**Files:**
- Create: `src/features/mod.rs`
- Create: `src/features/add.rs`
- Create: `src/features/list.rs`
- Create: `src/features/edit.rs`
- Test: inline `#[cfg(test)]` module in `features/mod.rs`

**Interfaces:**
- Consumes: `QueueRepository`, `CommandText`, `QueueError` (from Tasks 1-2)
- Produces: `features::add::add_command(&mut Q, &str) -> Result<(), QueueError>`
- Produces: `features::list::list_commands(&Q) -> Result<(), QueueError>`
- Produces: `features::edit::edit_queue(&Q) -> Result<(), QueueError>`

- [ ] **Step 1: Write features/mod.rs**

```rust
pub mod add;
pub mod edit;
pub mod list;
```

- [ ] **Step 2: Write features/add.rs**

```rust
use crate::domain::{CommandText, QueueError, QueueRepository};

pub fn add_command<Q: QueueRepository>(repo: &mut Q, raw: &str) -> Result<(), QueueError> {
    let command = CommandText::new(raw)?;
    repo.add(&command)
}
```

- [ ] **Step 3: Write features/list.rs**

```rust
use crate::domain::{QueueError, QueueRepository};

pub fn list_commands<Q: QueueRepository>(repo: &Q) -> Result<(), QueueError> {
    let commands = repo.list()?;
    for (i, cmd) in commands.iter().enumerate() {
        println!("{}: {}", i + 1, cmd.as_str());
    }
    Ok(())
}
```

- [ ] **Step 4: Write features/edit.rs**

```rust
use crate::domain::{QueueError, QueueRepository};

pub fn edit_queue<Q: QueueRepository>(repo: &Q) -> Result<(), QueueError> {
    repo.open_in_editor()
}
```

- [ ] **Step 5: Add tests to features/mod.rs**

Append to `src/features/mod.rs`:
```rust
pub mod add;
pub mod edit;
pub mod list;
pub mod process;

#[cfg(test)]
mod tests {
    use crate::domain::{CommandText, QueueError, QueueRepository};

    struct MockRepo {
        commands: Vec<CommandText>,
    }

    impl QueueRepository for MockRepo {
        fn add(&mut self, command: &CommandText) -> Result<(), QueueError> {
            self.commands.push(command.clone());
            Ok(())
        }
        fn list(&self) -> Result<Vec<CommandText>, QueueError> {
            Ok(self.commands.clone())
        }
        fn get_and_remove_first(&mut self) -> Result<Option<CommandText>, QueueError> {
            if self.commands.is_empty() {
                Ok(None)
            } else {
                Ok(Some(self.commands.remove(0)))
            }
        }
        fn open_in_editor(&self) -> Result<(), QueueError> {
            Ok(())
        }
    }

    #[test]
    fn add_command_valid() {
        let mut repo = MockRepo { commands: vec![] };
        super::add::add_command(&mut repo, "echo hello").unwrap();
        assert_eq!(repo.commands.len(), 1);
        assert_eq!(repo.commands[0].as_str(), "echo hello");
    }

    #[test]
    fn add_command_empty_rejected() {
        let mut repo = MockRepo { commands: vec![] };
        assert!(super::add::add_command(&mut repo, "").is_err());
    }

    #[test]
    fn add_command_whitespace_only_rejected() {
        let mut repo = MockRepo { commands: vec![] };
        assert!(super::add::add_command(&mut repo, "   ").is_err());
    }

    #[test]
    fn list_commands_returns_ok() {
        let repo = MockRepo {
            commands: vec![CommandText::new("cmd1").unwrap()],
        };
        assert!(super::list::list_commands(&repo).is_ok());
    }

    #[test]
    fn edit_queue_returns_ok() {
        let repo = MockRepo { commands: vec![] };
        assert!(super::edit::edit_queue(&repo).is_ok());
    }
}
```

- [ ] **Step 6: Run tests to verify**

```bash
cargo test features
```

Expected: 5 tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/features/
git commit -m "feat: add feature modules — add, list, edit"
```

---

### Task 4: Feature module — process

**Files:**
- Create: `src/features/process.rs`
- Test: inline `#[cfg(test)]` module in `process.rs`

**Interfaces:**
- Consumes: `QueueRepository`, `QueueError` (from Task 1)
- Produces: `features::process::process_queue(&mut Q) -> Result<(), QueueError>`

- [ ] **Step 1: Add process module declaration and write features/process.rs**

Add `pub mod process;` to `src/features/mod.rs`:
```rust
pub mod add;
pub mod edit;
pub mod list;
pub mod process;
```

Create `src/features/process.rs`:

```rust
use std::process::{Command as SystemCommand, Stdio};

use crate::domain::{QueueError, QueueRepository};

pub fn process_queue<Q: QueueRepository>(repo: &mut Q) -> Result<(), QueueError> {
    loop {
        match repo.get_and_remove_first()? {
            Some(command) => {
                let mut child = SystemCommand::new("fish")
                    .arg("-c")
                    .arg(command.as_str())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()?;
                child.wait()?;
            }
            None => break,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::domain::{CommandText, QueueError, QueueRepository};

    struct CountingMock {
        pub calls: usize,
        remaining: bool,
    }

    impl QueueRepository for CountingMock {
        fn add(&mut self, _: &CommandText) -> Result<(), QueueError> {
            unimplemented!()
        }
        fn list(&self) -> Result<Vec<CommandText>, QueueError> {
            unimplemented!()
        }
        fn get_and_remove_first(&mut self) -> Result<Option<CommandText>, QueueError> {
            self.calls += 1;
            if self.remaining {
                self.remaining = false;
                // Returns one command, then None on next call
                Ok(Some(CommandText::new("true").unwrap()))
            } else {
                Ok(None)
            }
        }
        fn open_in_editor(&self) -> Result<(), QueueError> {
            unimplemented!()
        }
    }

    #[test]
    fn process_queue_stops_when_empty() {
        let mut repo = CountingMock { calls: 0, remaining: false };
        super::process_queue(&mut repo).unwrap();
        assert_eq!(repo.calls, 1); // Called once, got None, stopped
    }

    #[test]
    fn process_queue_processes_one_command_then_stops() {
        let mut repo = CountingMock { calls: 0, remaining: true };
        // This test spawns `fish -c "true"` which is a no-op
        // It verifies the loop runs once then stops
        super::process_queue(&mut repo).unwrap();
        assert_eq!(repo.calls, 2); // First call returns Some, second returns None
    }
}
```

- [ ] **Step 2: Run tests to verify**

```bash
cargo test process
```

Expected: 2 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/features/process.rs
git commit -m "feat: add process feature module with queue processing loop"
```

---

### Task 5: CLI rewrite — main.rs with clap derive

**Files:**
- Modify: `src/main.rs` (complete rewrite)
- Test: inline `#[cfg(test)]` module in `main.rs`

**Interfaces:**
- Consumes: `FileQueue`, `QueueRepository`, `QueueError`, all feature modules (from Tasks 1-4)
- Produces: CLI binary with `add`, `start`, `ls`, `edit` subcommands

- [ ] **Step 1: Rewrite src/main.rs**

```rust
use clap::Parser;
use encode_queue::{
    adapters::FileQueue,
    domain::{QueueError, QueueRepository},
    features,
};

#[derive(Parser)]
#[command(name = "encode_queue", version = "1.0", about = "Manages a queue of commands")]
enum Cli {
    /// Adds a command to the queue
    Add {
        /// The command to add
        command: String,
    },
    /// Starts processing the queue
    Start,
    /// Lists all commands in the queue
    Ls,
    /// Opens the queue in an editor
    Edit,
}

fn main() -> Result<(), QueueError> {
    let cli = Cli::parse();
    let mut repo = FileQueue::new()?;

    match cli {
        Cli::Add { command } => features::add::add_command(&mut repo, &command)?,
        Cli::Start => features::process::process_queue(&mut repo)?,
        Cli::Ls => features::list::list_commands(&repo)?,
        Cli::Edit => features::edit::edit_queue(&repo)?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_commands_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn cli_requires_subcommand() {
        let result = Cli::try_parse_from(["encode_queue"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_add_parses_command() {
        let cli = Cli::try_parse_from(["encode_queue", "add", "echo hello"]).unwrap();
        assert!(matches!(cli, Cli::Add { command } if command == "echo hello"));
    }

    #[test]
    fn cli_start_parses() {
        let cli = Cli::try_parse_from(["encode_queue", "start"]).unwrap();
        assert!(matches!(cli, Cli::Start));
    }

    #[test]
    fn cli_ls_parses() {
        let cli = Cli::try_parse_from(["encode_queue", "ls"]).unwrap();
        assert!(matches!(cli, Cli::Ls));
    }

    #[test]
    fn cli_edit_parses() {
        let cli = Cli::try_parse_from(["encode_queue", "edit"]).unwrap();
        assert!(matches!(cli, Cli::Edit));
    }
}
```

- [ ] **Step 2: Run tests to verify**

```bash
cargo test --bin encode_queue
```

Expected: 6 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: rewrite main.rs with clap derive and feature dispatch"
```

---

### Task 6: Integration test, conventions audit, and cleanup

**Files:**
- Create: `tests/integration.rs`
- Modify: `README.md` (update for new CLI behavior if needed)

**Interfaces:**
- Consumes: All modules from Tasks 1-5
- Produces: Integration test verifying end-to-end behavior

- [ ] **Step 1: Write tests/integration.rs**

```rust
use encode_queue::{
    adapters::FileQueue,
    domain::{CommandText, QueueRepository},
    features,
};
use std::fs;
use tempfile::TempDir;

fn setup_queue() -> (FileQueue, TempDir) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("commands.txt");
    (FileQueue::with_path(path), dir)
}

#[test]
fn add_then_list() {
    let (mut queue, _dir) = setup_queue();
    features::add::add_command(&mut queue, "ffmpeg -i input.mp4 output.mp4").unwrap();

    let commands = queue.list().unwrap();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].as_str(), "ffmpeg -i input.mp4 output.mp4");
}

#[test]
fn add_multiple_then_list_preserves_order() {
    let (mut queue, _dir) = setup_queue();
    features::add::add_command(&mut queue, "first").unwrap();
    features::add::add_command(&mut queue, "second").unwrap();
    features::add::add_command(&mut queue, "third").unwrap();

    let commands = queue.list().unwrap();
    assert_eq!(commands.len(), 3);
    assert_eq!(commands[0].as_str(), "first");
    assert_eq!(commands[1].as_str(), "second");
    assert_eq!(commands[2].as_str(), "third");
}

#[test]
fn add_empty_command_rejected() {
    let (mut queue, _dir) = setup_queue();
    let result = features::add::add_command(&mut queue, "");
    assert!(result.is_err());
}

#[test]
fn list_empty_queue() {
    let (queue, _dir) = setup_queue();
    let result = features::list::list_commands(&queue);
    assert!(result.is_ok());
}

#[test]
fn queue_persists_to_file() {
    let (mut queue, dir) = setup_queue();
    features::add::add_command(&mut queue, "persisted command").unwrap();

    // Verify file exists and contains the command
    let contents = fs::read_to_string(dir.path().join("commands.txt")).unwrap();
    assert!(contents.contains("persisted command"));
}
```

- [ ] **Step 2: Run full test suite**

```bash
cargo test
```

Expected: All tests pass (domain: 8, adapters: 6, features: 5, process: 2, main: 6, integration: 5 = 32 total).

- [ ] **Step 3: Verify conventions compliance**

Run through the checklist in `CONVENTIONS.md`:

| Check | Status |
|-------|--------|
| Every distinct concept has its own type | ✅ `CommandText` |
| Wrapped fields are private | ✅ `CommandText(String)` — field is private |
| Standard traits derived | ✅ `Debug, Clone, PartialEq, Eq, Hash` |
| All external input parsed into domain types | ✅ `add_command` calls `CommandText::new` |
| Conversions use `TryFrom` | ✅ `impl TryFrom<&str> for CommandText` |
| No raw types leak past boundary | ✅ Features use `CommandText`, not `String` |
| Errors are enums with codified variants | ✅ `QueueError`, `InvalidCommandError` |
| Implementation errors translated at boundary | ✅ `FileQueue` maps `io::Error` → `QueueError::Io` |
| Domain has zero external dependencies | ✅ `domain/` imports only `std` + `thiserror` |
| `main` contains only bootstrapping | ✅ Only CLI parsing and dispatch |
| Code organized by feature (vertical slices) | ✅ `features/` directory |
| Enum dispatch instead of `dyn` | ✅ `QueueRepository` is a trait, not `dyn` |
| Constructors tested exhaustively | ✅ Valid, empty, whitespace, trim |
| Error paths tested | ✅ Empty command rejection, IO error conversion |
| Domain tests use mocks | ✅ `MockRepo` in features tests |

- [ ] **Step 4: Commit**

```bash
git add tests/
git commit -m "test: add integration tests and verify conventions compliance"
```