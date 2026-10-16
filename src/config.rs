//! Environment configuration (`HOME`, `EDITOR`, `ENCODE_QUEUE_DIR`,
//! `ENCODE_QUEUE_SHELL`) → domain newtypes.
//!
//! `Config` is the single origin of the queue `QueuePath`, the `Editor`, and
//! the `Shell` (V-ARCH-05): it reads the environment, converts the raw
//! `HOME`/`EDITOR`/`ENCODE_QUEUE_DIR`/`ENCODE_QUEUE_SHELL` values into the
//! domain newtypes at the boundary (V-TY-03), and reports a missing (or
//! non-Unicode) `HOME` as `ConfigError::HomeUnset` — a `Result` error, never a
//! panic (V-ERR-01). `ENCODE_QUEUE_DIR` (P6) optionally overrides the queue
//! directory: the queue file is then `<ENCODE_QUEUE_DIR>/commands.txt` instead
//! of the legacy `$HOME/.config/encode_queue/commands.txt`; an unset, empty,
//! or whitespace-only value is treated as unset (mirroring the `Editor`
//! empty→default convention), and a non-Unicode value is
//! `ConfigError::QueueDirInvalid`. `ENCODE_QUEUE_SHELL` (P6.F2) selects the
//! shell `start` spawns: an unset, empty, or whitespace-only value defaults to
//! `fish`, a valid name (`fish`/`bash`/`sh`) selects that shell, and a
//! non-Unicode or unknown value is `ConfigError::ShellInvalid`.

use std::ffi::OsStr;
use std::path::PathBuf;

use crate::domain::queue::{Editor, QueuePath};
use crate::domain::shell::Shell;

/// Application configuration parsed from the environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    queue_path: QueuePath,
    editor: Editor,
    shell: Shell,
}

impl Config {
    /// Read `HOME`, `EDITOR`, `ENCODE_QUEUE_DIR` and `ENCODE_QUEUE_SHELL` from
    /// the environment and convert them into domain newtypes.
    ///
    /// `HOME` is required in all cases: a missing (or non-Unicode) `HOME`
    /// yields `Err(ConfigError::HomeUnset)` rather than panicking — even when
    /// `ENCODE_QUEUE_DIR` is set. `EDITOR` defaults to `nvim` when unset or
    /// empty. `ENCODE_QUEUE_DIR` overrides the queue directory (see
    /// [`Self::from_vars`]). `ENCODE_QUEUE_SHELL` selects the shell (see
    /// [`Self::from_vars`]).
    pub fn from_env() -> Result<Self, ConfigError> {
        let home = std::env::var_os("HOME");
        let editor = std::env::var("EDITOR").ok();
        let queue_dir = std::env::var_os("ENCODE_QUEUE_DIR");
        let shell = std::env::var_os("ENCODE_QUEUE_SHELL");
        Self::from_vars(
            home.as_deref(),
            editor.as_deref(),
            queue_dir.as_deref(),
            shell.as_deref(),
        )
    }

    /// Convert raw `HOME`/`EDITOR`/`ENCODE_QUEUE_DIR` environment values into
    /// domain newtypes.
    ///
    /// Pure with respect to process state — the env values are parameters,
    /// not read from the process environment — so it can be unit-tested
    /// deterministically without env mutation. A `None` or non-Unicode `home`
    /// yields `Err(ConfigError::HomeUnset)` in **all** cases (the override
    /// does not relax the `HOME` requirement); a `None` or empty `editor`
    /// defaults to `nvim`.
    ///
    /// `ENCODE_QUEUE_DIR` semantics (P6): a `Some` value that is non-empty
    /// valid Unicode (and not whitespace-only) wins over `HOME` — the queue
    /// file is `<queue_dir>/commands.txt`. An unset, empty, or whitespace-only
    /// value is treated as unset (empty mirrors the `Editor` empty→default
    /// convention) and the legacy `$HOME/.config/encode_queue/commands.txt`
    /// path is used. A set value that is not valid Unicode yields
    /// `Err(ConfigError::QueueDirInvalid)`.
    ///
    /// `ENCODE_QUEUE_SHELL` semantics (P6.F2, mirroring the `Editor`
    /// empty→default convention): an unset, empty, or whitespace-only value
    /// defaults to `Shell::Fish`. A valid name (`fish`, `bash`, `sh`) selects
    /// that shell. A set value that is not valid Unicode yields
    /// `Err(ConfigError::ShellInvalid)` with the lossy string; a known-invalid
    /// (non-Unicode-free) name yields `Err(ConfigError::ShellInvalid)` with the
    /// input string.
    pub fn from_vars(
        home: Option<&OsStr>,
        editor: Option<&str>,
        queue_dir: Option<&OsStr>,
        shell: Option<&OsStr>,
    ) -> Result<Self, ConfigError> {
        let home = home.and_then(OsStr::to_str).ok_or(ConfigError::HomeUnset)?;
        // `ENCODE_QUEUE_DIR`: empty/whitespace-only → treated as unset;
        // non-Unicode → `QueueDirInvalid`; otherwise the override wins.
        let override_dir = match queue_dir.and_then(OsStr::to_str) {
            Some(dir) if !dir.trim().is_empty() => Some(dir),
            Some(_) => None,
            None => match queue_dir {
                Some(dir) if !dir.is_empty() => return Err(ConfigError::QueueDirInvalid),
                _ => None,
            },
        };
        let queue_path = match override_dir {
            Some(dir) => QueuePath::new(PathBuf::from(dir).join("commands.txt")),
            None => QueuePath::new(PathBuf::from(home).join(".config/encode_queue/commands.txt")),
        };
        // `Editor::new` applies the `nvim` default internally (`None` or
        // empty → `nvim`); `config.rs` passes `None` in the empty/unset arm.
        let editor = match editor {
            Some(name) if !name.is_empty() => Editor::new(Some(name)),
            _ => Editor::new(None),
        };
        // `ENCODE_QUEUE_SHELL` (P6.F2): empty/whitespace-only → default
        // `Shell::Fish`; non-Unicode (non-empty) → `ShellInvalid` (lossy);
        // otherwise the named shell, or `ShellInvalid` for a known-invalid
        // name (carrying the input).
        let shell = match shell.and_then(OsStr::to_str) {
            Some(name) if !name.trim().is_empty() => {
                Shell::new(name).map_err(|e| ConfigError::ShellInvalid(e.input().to_owned()))?
            }
            Some(_) => Shell::Fish,
            None => match shell {
                Some(os) if !os.is_empty() => {
                    return Err(ConfigError::ShellInvalid(os.to_string_lossy().into_owned()))
                }
                _ => Shell::Fish,
            },
        };
        Ok(Self {
            queue_path,
            editor,
            shell,
        })
    }

    /// Borrow the queue-file path.
    pub fn queue_path(&self) -> &QueuePath {
        &self.queue_path
    }

    /// Borrow the editor.
    pub fn editor(&self) -> &Editor {
        &self.editor
    }

    /// Borrow the selected shell.
    pub fn shell(&self) -> &Shell {
        &self.shell
    }
}

/// Errors raised while parsing the environment into a [`Config`].
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// `HOME` is not set (or is not valid Unicode), so the queue path cannot
    /// be derived.
    #[error("HOME environment variable is not set")]
    HomeUnset,
    /// `ENCODE_QUEUE_DIR` is set but is not valid Unicode, so it cannot be
    /// used as the queue directory. Routed through `main`'s generic
    /// config-error arm: the message is printed and the process exits with
    /// 1 (same treatment as `HomeUnset`); no dedicated exit-code policy.
    #[error("ENCODE_QUEUE_DIR is not valid Unicode")]
    QueueDirInvalid,
    /// `ENCODE_QUEUE_SHELL` is set to a value that is not a valid shell name
    /// (non-Unicode, or an unknown name). Routed through `main`'s usage-class
    /// error arm: the message is printed and the process exits with 2 (P6.F2
    /// spec), distinct from the generic config-error exit 1.
    #[error("ENCODE_QUEUE_SHELL '{0}' is not a valid shell: expected fish, bash, or sh")]
    ShellInvalid(String),
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

    /// Fixed queue file name under the `ENCODE_QUEUE_DIR` override
    /// (must match `Config::from_vars`).
    const OVERRIDE_QUEUE_FILE: &str = "commands.txt";

    fn snapshot_env() -> (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) {
        (
            std::env::var("HOME").ok(),
            std::env::var("EDITOR").ok(),
            std::env::var("ENCODE_QUEUE_DIR").ok(),
            std::env::var("ENCODE_QUEUE_SHELL").ok(),
        )
    }

    fn restore_env(
        home: Option<String>,
        editor: Option<String>,
        queue_dir: Option<String>,
        shell: Option<String>,
    ) {
        match home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match editor {
            Some(v) => std::env::set_var("EDITOR", v),
            None => std::env::remove_var("EDITOR"),
        }
        match queue_dir {
            Some(v) => std::env::set_var("ENCODE_QUEUE_DIR", v),
            None => std::env::remove_var("ENCODE_QUEUE_DIR"),
        }
        match shell {
            Some(v) => std::env::set_var("ENCODE_QUEUE_SHELL", v),
            None => std::env::remove_var("ENCODE_QUEUE_SHELL"),
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
        let result = Config::from_vars(None, None, None, None);
        assert!(
            matches!(result, Err(ConfigError::HomeUnset)),
            "expected ConfigError::HomeUnset, got {result:?}"
        );
    }

    /// A non-Unicode `HOME` is the same `Result` error, not a panic.
    #[test]
    fn from_vars_home_non_unicode_is_an_error() {
        let home: std::ffi::OsString = OsStringExt::from_vec(vec![0xff]);
        let result = Config::from_vars(Some(home.as_os_str()), None, None, None);
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

        let cfg = must_be_config(Config::from_vars(Some(home), None, None, None));
        assert_eq!(cfg.queue_path().as_path(), expected_path);
        assert_eq!(cfg.editor().as_str(), "nvim");

        let cfg = must_be_config(Config::from_vars(Some(home), Some(""), None, None));
        assert_eq!(cfg.queue_path().as_path(), expected_path);
        assert_eq!(cfg.editor().as_str(), "nvim");
    }

    /// A set, non-empty, non-whitespace `ENCODE_QUEUE_DIR` wins over `HOME`
    /// (P6): the queue file is `<queue_dir>/commands.txt`.
    #[test]
    fn from_vars_queue_dir_override_wins_over_home() {
        let home = std::ffi::OsStr::new("/tmp/fake-home");
        let queue_dir = std::ffi::OsStr::new("/tmp/custom-queue");
        let cfg = must_be_config(Config::from_vars(Some(home), None, Some(queue_dir), None));
        assert_eq!(
            cfg.queue_path().as_path(),
            std::path::Path::new("/tmp/custom-queue").join(OVERRIDE_QUEUE_FILE)
        );
    }

    /// An empty or whitespace-only `ENCODE_QUEUE_DIR` is treated as unset:
    /// the legacy `HOME`-based path is used.
    #[test]
    fn from_vars_empty_or_whitespace_queue_dir_falls_back_to_home() {
        let home = std::ffi::OsStr::new("/tmp/fake-home");
        let expected_path = std::path::Path::new("/tmp/fake-home").join(QUEUE_FILE_SUFFIX);

        for queue_dir in [std::ffi::OsStr::new(""), std::ffi::OsStr::new("   ")] {
            let cfg = must_be_config(Config::from_vars(Some(home), None, Some(queue_dir), None));
            assert_eq!(cfg.queue_path().as_path(), expected_path);
        }
    }

    /// A non-Unicode `ENCODE_QUEUE_DIR` is a `Result` error, not a panic.
    #[test]
    fn from_vars_queue_dir_non_unicode_is_an_error() {
        let home = std::ffi::OsStr::new("/tmp/fake-home");
        let queue_dir: std::ffi::OsString = OsStringExt::from_vec(vec![0xff]);
        let result = Config::from_vars(Some(home), None, Some(queue_dir.as_os_str()), None);
        assert!(
            matches!(result, Err(ConfigError::QueueDirInvalid)),
            "expected ConfigError::QueueDirInvalid, got {result:?}"
        );
    }

    /// `HOME` remains required even when the override is set: a missing
    /// `HOME` is still `HomeUnset` (the override does not relax the `HOME`
    /// requirement).
    #[test]
    fn from_vars_queue_dir_override_still_requires_home() {
        let queue_dir = std::ffi::OsStr::new("/tmp/custom-queue");
        let result = Config::from_vars(None, None, Some(queue_dir), None);
        assert!(
            matches!(result, Err(ConfigError::HomeUnset)),
            "expected ConfigError::HomeUnset, got {result:?}"
        );
    }

    /// A set, non-empty `EDITOR` is parsed into the `Editor` newtype verbatim.
    #[test]
    fn from_vars_uses_configured_editor() {
        let home = std::ffi::OsStr::new("/tmp/fake-home");
        let cfg = must_be_config(Config::from_vars(Some(home), Some("vim"), None, None));
        assert_eq!(
            cfg.queue_path().as_path(),
            std::path::Path::new("/tmp/fake-home").join(QUEUE_FILE_SUFFIX)
        );
        assert_eq!(cfg.editor().as_str(), "vim");
    }

    /// `Config::from_env` reads `HOME`/`EDITOR`/`ENCODE_QUEUE_DIR`/
    /// `ENCODE_QUEUE_SHELL` through the real `env::var`/`env::var_os` path.
    /// The only env-mutating test: it takes `ENV_LOCK` and restores all four
    /// variables on exit.
    #[test]
    fn from_env_reads_home_editor_and_queue_dir_from_the_environment() {
        let guard = match ENV_LOCK.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };

        let (home, editor, queue_dir, shell) = snapshot_env();

        std::env::set_var("HOME", "/tmp/fake-home");
        std::env::set_var("EDITOR", "envtest-editor");
        let cfg = must_be_config(Config::from_env());
        assert_eq!(
            cfg.queue_path().as_path(),
            std::path::Path::new("/tmp/fake-home").join(QUEUE_FILE_SUFFIX)
        );
        assert_eq!(cfg.editor().as_str(), "envtest-editor");

        // `ENCODE_QUEUE_DIR` overrides the queue directory (P6).
        std::env::set_var("ENCODE_QUEUE_DIR", "/tmp/envtest-queue");
        let cfg = must_be_config(Config::from_env());
        assert_eq!(
            cfg.queue_path().as_path(),
            std::path::Path::new("/tmp/envtest-queue").join(OVERRIDE_QUEUE_FILE)
        );

        // `ENCODE_QUEUE_SHELL` selects the shell (P6.F2).
        std::env::set_var("ENCODE_QUEUE_SHELL", "sh");
        let cfg = must_be_config(Config::from_env());
        assert_eq!(cfg.shell(), &Shell::Sh);

        std::env::remove_var("HOME");
        let result = Config::from_env();
        assert!(
            matches!(result, Err(ConfigError::HomeUnset)),
            "expected ConfigError::HomeUnset (still required with the override set), got {result:?}"
        );

        restore_env(home, editor, queue_dir, shell);
        drop(guard);
    }

    /// A valid `ENCODE_QUEUE_SHELL` value parses to the matching `Shell`
    /// (P6.F2).
    #[test]
    fn from_vars_shell_valid_selects_that_shell() {
        let home = std::ffi::OsStr::new("/tmp/fake-home");
        let shell = std::ffi::OsStr::new("sh");
        let cfg = must_be_config(Config::from_vars(Some(home), None, None, Some(shell)));
        assert_eq!(cfg.shell(), &Shell::Sh);
    }

    /// An unset, empty, or whitespace-only `ENCODE_QUEUE_SHELL` defaults to
    /// `Shell::Fish` (P6.F2, mirroring the `Editor` empty→default convention;
    /// the whitespace-only case is P7.FIX01, minor finding 6).
    #[test]
    fn from_vars_shell_unset_empty_or_whitespace_defaults_to_fish() {
        let home = std::ffi::OsStr::new("/tmp/fake-home");
        for shell in [
            None,
            Some(std::ffi::OsStr::new("")),
            Some(std::ffi::OsStr::new("   ")),
        ] {
            let cfg = must_be_config(Config::from_vars(Some(home), None, None, shell));
            assert_eq!(cfg.shell(), &Shell::Fish);
        }
    }

    /// A known-invalid `ENCODE_QUEUE_SHELL` name is `ShellInvalid` carrying the
    /// input (P6.F2).
    #[test]
    fn from_vars_shell_known_invalid_is_shell_invalid() {
        let home = std::ffi::OsStr::new("/tmp/fake-home");
        let shell = std::ffi::OsStr::new("zsh");
        let result = Config::from_vars(Some(home), None, None, Some(shell));
        assert!(
            matches!(result, Err(ConfigError::ShellInvalid(_))),
            "expected ConfigError::ShellInvalid, got {result:?}"
        );
    }

    /// A non-Unicode `ENCODE_QUEUE_SHELL` is `ShellInvalid` carrying the lossy
    /// string, not a panic (P6.F2).
    #[test]
    fn from_vars_shell_non_unicode_is_shell_invalid() {
        let home = std::ffi::OsStr::new("/tmp/fake-home");
        let shell: std::ffi::OsString = OsStringExt::from_vec(vec![0xff]);
        let result = Config::from_vars(Some(home), None, None, Some(shell.as_os_str()));
        assert!(
            matches!(result, Err(ConfigError::ShellInvalid(_))),
            "expected ConfigError::ShellInvalid, got {result:?}"
        );
    }
}
