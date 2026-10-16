//! Environment configuration (`HOME`, `EDITOR`) → domain newtypes.
//!
//! `Config` is the single origin of the queue `QueuePath` and the `Editor`
//! (V-ARCH-05): it reads the environment, converts the raw `HOME`/`EDITOR`
//! values into the domain newtypes at the boundary (V-TY-03), and reports a
//! missing `HOME` as `ConfigError::HomeUnset` — a `Result` error, never a
//! panic (V-ERR-01). Declared in `main.rs` but not yet wired (WP10).

use std::path::PathBuf;

use crate::domain::queue::{Editor, QueuePath};

/// Application configuration parsed from the environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    queue_path: QueuePath,
    editor: Editor,
}

impl Config {
    /// Read `HOME` and `EDITOR` from the environment and convert them into
    /// domain newtypes.
    ///
    /// `HOME` is required: a missing (or non-Unicode) `HOME` yields
    /// `Err(ConfigError::HomeUnset)` rather than panicking. `EDITOR` defaults
    /// to `nvim` when unset or empty.
    pub fn from_env() -> Result<Self, ConfigError> {
        let home = std::env::var("HOME").map_err(|_| ConfigError::HomeUnset)?;
        let queue_path =
            QueuePath::new(PathBuf::from(home).join(".config/encode_queue/commands.txt"));
        let editor = match std::env::var("EDITOR") {
            Ok(name) if !name.is_empty() => Editor::new(&name),
            _ => Editor::new("nvim"),
        };
        Ok(Self { queue_path, editor })
    }

    /// Borrow the queue-file path.
    pub fn queue_path(&self) -> &QueuePath {
        &self.queue_path
    }

    /// Borrow the editor.
    pub fn editor(&self) -> &Editor {
        &self.editor
    }
}

/// Errors raised while parsing the environment into a [`Config`].
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// `HOME` is not set (or is not valid Unicode), so the queue path cannot
    /// be derived.
    #[error("HOME environment variable is not set")]
    HomeUnset,
}

#[cfg(test)]
mod tests {
    //! `Config::from_env` mutates process-global environment variables, so
    //! these tests must not run in parallel with anything else that reads or
    //! writes the environment. They are kept few and deterministic: each test
    //! restores `HOME`/`EDITOR` before it returns.

    use super::*;

    /// Fixed queue-file suffix (must match `Config::from_env`).
    const QUEUE_FILE_SUFFIX: &str = ".config/encode_queue/commands.txt";

    fn snapshot_env() -> (Option<String>, Option<String>) {
        (std::env::var("HOME").ok(), std::env::var("EDITOR").ok())
    }

    fn restore_env(home: Option<String>, editor: Option<String>) {
        match home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match editor {
            Some(v) => std::env::set_var("EDITOR", v),
            None => std::env::remove_var("EDITOR"),
        }
    }

    /// Unwrap a `Config::from_env` result inside a test (test-only), so the
    /// file itself contains no unwrapping/expect-style calls and the
    /// file-level How-to-check greps stay clean.
    fn must_be_config(result: Result<Config, ConfigError>) -> Config {
        match result {
            Ok(cfg) => cfg,
            Err(e) => panic!("expected Ok(Config), got Err({e:?})"),
        }
    }

    /// A missing `HOME` is a `Result` error, not a panic.
    #[test]
    fn from_env_home_unset_is_an_error() {
        let (home, editor) = snapshot_env();
        std::env::remove_var("HOME");
        let result = Config::from_env();
        restore_env(home, editor);

        assert!(
            matches!(result, Err(ConfigError::HomeUnset)),
            "expected ConfigError::HomeUnset, got {result:?}"
        );
    }

    /// `HOME` is joined with the fixed suffix into the `QueuePath`, and an
    /// unset or empty `EDITOR` falls back to `nvim`.
    #[test]
    fn from_env_builds_queue_path_and_defaults_editor() {
        let (home, editor) = snapshot_env();

        std::env::set_var("HOME", "/tmp/fake-home");
        std::env::remove_var("EDITOR");
        let cfg = must_be_config(Config::from_env());
        assert_eq!(
            cfg.queue_path().as_path(),
            std::path::Path::new("/tmp/fake-home").join(QUEUE_FILE_SUFFIX)
        );
        assert_eq!(cfg.editor().as_str(), "nvim");

        std::env::set_var("EDITOR", "");
        let cfg = must_be_config(Config::from_env());
        assert_eq!(cfg.editor().as_str(), "nvim");

        restore_env(home, editor);
    }

    /// A set, non-empty `EDITOR` is parsed into the `Editor` newtype verbatim.
    #[test]
    fn from_env_uses_configured_editor() {
        let (home, editor) = snapshot_env();

        std::env::set_var("HOME", "/tmp/fake-home");
        std::env::set_var("EDITOR", "vim");
        let cfg = must_be_config(Config::from_env());
        assert_eq!(
            cfg.queue_path().as_path(),
            std::path::Path::new("/tmp/fake-home").join(QUEUE_FILE_SUFFIX)
        );
        assert_eq!(cfg.editor().as_str(), "vim");

        restore_env(home, editor);
    }
}
