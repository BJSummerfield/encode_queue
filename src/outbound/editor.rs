//! `EditorProcess` outbound adapter (the editor boundary).
//!
//! WP06: implements the `EditorLauncher` port. The editor binary name comes
//! from the injected `Editor` domain newtype (WP02) — the adapter never
//! reads the environment; `Config` (WP07) supplies the value at WP10
//! wiring. The queue file path crosses the port as the `QueuePath` newtype
//! (V-TY-05), and the queue path is passed as the
//! editor's single argument.
//!
//! Error mapping (V-ERR-05): a failed `spawn` is the named
//! `EditorError::Spawn` variant and a failed `wait` is `EditorError::Other`;
//! neither aborts the process. The legacy CLI's editor exit status is
//! deliberately discarded — `open` returns `Ok(())` for any exit code,
//! preserving the legacy behaviour that only spawn/wait I/O errors were
//! failures.

use std::process::Command;

use crate::domain::queue::{Editor, QueuePath};
use crate::ports::editor::{EditorError, EditorLauncher};

/// Process-backed [`EditorLauncher`] that spawns the configured editor on
/// the queue file.
pub struct EditorProcess {
    editor: Editor,
}

impl EditorProcess {
    /// Wrap the editor to launch; the `nvim`-default semantics already live
    /// in the `Editor` newtype (WP02).
    pub fn new(editor: Editor) -> Self {
        Self { editor }
    }
}

impl EditorLauncher for EditorProcess {
    fn open(&self, at: &QueuePath) -> Result<(), EditorError> {
        let mut child = Command::new(self.editor.as_str())
            .arg(at.as_path())
            .spawn()
            .map_err(EditorError::Spawn)?;
        child.wait().map_err(EditorError::Other)?;
        // The legacy CLI discards the editor's exit status (it only failed
        // on spawn/wait I/O errors); preserve that — a non-zero exit is Ok
        // here.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    // Each test gets a unique temp subdirectory under `std::env::temp_dir()`;
    // no test touches `$HOME` or the process environment (TST-01) — the
    // editor is constructor-injected.
    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> PathBuf {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("encode_queue_wp06_{}_{id}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Write a shell script at `dir` and make it executable (0o755).
    ///
    /// The bounded CPU spin before returning is a workaround for a
    /// host-kernel quirk observed in this container (Linux 6.18, NixOS
    /// nspawn): when sibling `cargo test` threads are concurrently writing
    /// their own temp files, exec-ing a script that was written
    /// microseconds ago can fail with `ETXTBSY` ("Text file busy") — a
    /// transient, host-level error that has nothing to do with
    /// `EditorProcess`. The exec is reliably successful once the write has
    /// settled (~1 ms), so the helper waits out that window. The spin is
    /// time-bounded (never blocks indefinitely) and only delays the test by
    /// ~1 ms.
    fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).unwrap();
        let written = std::time::Instant::now();
        while written.elapsed().as_millis() < 1 {
            std::hint::spin_loop();
        }
        path
    }

    /// Build an `EditorProcess` whose editor is the fake script at `script`,
    /// and a queue path `dir/commands.txt` (created so the fake can inspect
    /// its argument).
    fn fixture(script: &Path, dir: &Path) -> (EditorProcess, QueuePath, PathBuf) {
        let queue_path = dir.join("commands.txt");
        std::fs::write(&queue_path, "").unwrap();
        let launcher = EditorProcess::new(Editor::new(Some(script.to_str().unwrap())));
        (launcher, QueuePath::new(queue_path.clone()), queue_path)
    }

    fn cleanup(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn open_succeeds_and_passes_the_queue_path_as_the_single_argument() {
        let dir = temp_dir();
        let script = write_script(
            &dir,
            "editor.sh",
            r#"printf '%s\n' "$1" > "$(dirname "$1")/editor_arg.txt""#,
        );
        let (launcher, queue, queue_path) = fixture(&script, &dir);

        assert_eq!(launcher.open(&queue).unwrap(), ());

        // Pins: the queue path is the editor's single (and only) argument.
        let arg = std::fs::read_to_string(dir.join("editor_arg.txt")).unwrap();
        assert_eq!(arg.trim_end(), queue_path.display().to_string());
        cleanup(&dir);
    }

    #[test]
    fn open_discards_a_nonzero_exit_status() {
        let dir = temp_dir();
        let script = write_script(&dir, "editor.sh", "exit 3");
        let (launcher, queue, _) = fixture(&script, &dir);

        // Legacy behaviour preserved: only spawn/wait I/O errors fail; the
        // editor's exit status is discarded.
        assert_eq!(launcher.open(&queue).unwrap(), ());
        cleanup(&dir);
    }

    #[test]
    fn open_with_a_missing_editor_binary_is_spawn_error() {
        let dir = temp_dir();
        let launcher =
            EditorProcess::new(Editor::new(Some("/nonexistent/editor-binary-xyz-12345")));
        let queue = QueuePath::new(dir.join("commands.txt"));

        let err = launcher.open(&queue).unwrap_err();
        assert!(matches!(err, EditorError::Spawn(_)));
        cleanup(&dir);
    }

    // `Editor` newtype defaults (None/empty → `nvim`) are covered by the
    // WP02 domain tests and are not re-tested here.
}
