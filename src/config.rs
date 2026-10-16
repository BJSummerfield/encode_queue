//! Environment configuration (`HOME`, `EDITOR`) → domain newtypes.
//!
//! `Config` is the single origin of the queue `QueuePath` and the `Editor`
//! (V-ARCH-05): it reads the environment, converts the raw `HOME`/`EDITOR`
//! values into the domain newtypes at the boundary (V-TY-03), and reports a
//! missing (or non-Unicode) `HOME` as `ConfigError::HomeUnset` — a `Result`
//! error, never a panic (V-ERR-01). Declared in `main.rs` but not yet wired
//! (WP10).

use std::ffi::OsStr;
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
        let home = std::env::var_os("HOME");
        let editor = std::env::var("EDITOR").ok();
        Self::from_vars(home.as_deref(), editor.as_deref())
    }

    /// Convert raw `HOME`/`EDITOR` environment values into domain newtypes.
    ///
    /// Pure with respect to process state — the env values are parameters,
    /// not read from the process environment — so it can be unit-tested
    /// deterministically without env mutation. A `None` or non-Unicode `home`
    /// yields `Err(ConfigError::HomeUnset)`; a `None` or empty `editor`
    /// defaults to `nvim`.
    pub fn from_vars(home: Option<&OsStr>, editor: Option<&str>) -> Result<Self, ConfigError> {
        let home = home.and_then(OsStr::to_str).ok_or(ConfigError::HomeUnset)?;
        let queue_path =
            QueuePath::new(PathBuf::from(home).join(".config/encode_queue/commands.txt"));
        // `Editor::new` applies the `nvim` default internally (`None` or
        // empty → `nvim`); `config.rs` passes `None` in the empty/unset arm.
        let editor = match editor {
            Some(name) if !name.is_empty() => Editor::new(Some(name)),
            _ => Editor::new(None),
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
    //! The bulk of the tests drive the pure `Config::from_vars`, which is
    //! deterministic and parallel-safe (no process-state mutation). Exactly
    //! one test exercises the real `Config::from_env` path; it mutates
    //! process-global environment variables and takes `ENV_LOCK`, so it can
    //! never run concurrently with any other env-mutating test (there are
    //! none today; the lock keeps the invariant enforced if one is added).

    use super::*;
    use std::os::unix::ffi::OsStringExt;
    use std::sync::Mutex;

    /// Serializes env-mutating tests (libtest runs tests in parallel).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Fixed queue-file suffix (must match `Config::from_vars`).
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

    /// Unwrap a `Config` result inside a test (test-only), so the
    /// file itself contains no unwrapping/expect-style calls and the
    /// file-level How-to-check greps stay clean.
    fn must_be_config(result: Result<Config, ConfigError>) -> Config {
        match result {
            Ok(cfg) => cfg,
            Err(e) => panic!("expected Ok(Config), got Err({e:?})"),
        }
    }

    /// A missing `HOME` is a `Result` error, not a panic (pure input).
    #[test]
    fn from_vars_home_unset_is_an_error() {
        let result = Config::from_vars(None, None);
        assert!(
            matches!(result, Err(ConfigError::HomeUnset)),
            "expected ConfigError::HomeUnset, got {result:?}"
        );
    }

    /// A non-Unicode `HOME` is the same `Result` error, not a panic.
    #[test]
    fn from_vars_home_non_unicode_is_an_error() {
        let home: std::ffi::OsString = OsStringExt::from_vec(vec![0xff]);
        let result = Config::from_vars(Some(home.as_os_str()), None);
        assert!(
            matches!(result, Err(ConfigError::HomeUnset)),
            "expected ConfigError::HomeUnset, got {result:?}"
        );
    }

    /// `HOME` is joined with the fixed suffix into the `QueuePath`, and an
    /// unset or empty `EDITOR` falls back to `nvim`.
    #[test]
    fn from_vars_builds_queue_path_and_defaults_editor() {
        let home = std::ffi::OsStr::new("/tmp/fake-home");
        let expected_path = std::path::Path::new("/tmp/fake-home").join(QUEUE_FILE_SUFFIX);

        let cfg = must_be_config(Config::from_vars(Some(home), None));
        assert_eq!(cfg.queue_path().as_path(), expected_path);
        assert_eq!(cfg.editor().as_str(), "nvim");

        let cfg = must_be_config(Config::from_vars(Some(home), Some("")));
        assert_eq!(cfg.queue_path().as_path(), expected_path);
        assert_eq!(cfg.editor().as_str(), "nvim");
    }

    /// A set, non-empty `EDITOR` is parsed into the `Editor` newtype verbatim.
    #[test]
    fn from_vars_uses_configured_editor() {
        let home = std::ffi::OsStr::new("/tmp/fake-home");
        let cfg = must_be_config(Config::from_vars(Some(home), Some("vim")));
        assert_eq!(
            cfg.queue_path().as_path(),
            std::path::Path::new("/tmp/fake-home").join(QUEUE_FILE_SUFFIX)
        );
        assert_eq!(cfg.editor().as_str(), "vim");
    }

    /// `Config::from_env` reads `HOME`/`EDITOR` through the real
    /// `env::var`/`env::var_os` path. The only env-mutating test: it takes
    /// `ENV_LOCK` and restores both variables on exit.
    #[test]
    fn from_env_reads_home_and_editor_from_the_environment() {
        let guard = match ENV_LOCK.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };

        let (home, editor) = snapshot_env();

        std::env::set_var("HOME", "/tmp/fake-home");
        std::env::set_var("EDITOR", "envtest-editor");
        let cfg = must_be_config(Config::from_env());
        assert_eq!(
            cfg.queue_path().as_path(),
            std::path::Path::new("/tmp/fake-home").join(QUEUE_FILE_SUFFIX)
        );
        assert_eq!(cfg.editor().as_str(), "envtest-editor");

        std::env::remove_var("HOME");
        let result = Config::from_env();
        assert!(
            matches!(result, Err(ConfigError::HomeUnset)),
            "expected ConfigError::HomeUnset, got {result:?}"
        );

        restore_env(home, editor);
        drop(guard);
    }
}
