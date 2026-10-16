//! `FishRunner` outbound adapter (the process boundary).
//!
//! WP05: `FishRunner` implements the `CommandRunner` port by running
//! `fish -c <entry>` with inherited stdio. `std::process::Command` is
//! confined to this file (ARCH-03); every failure is translated into the
//! port's `RunError` variants — spawn failures become `RunError::Spawn`
//! and wait I/O failures the `RunError::Other` catch-all (ERR-04).
//!
//! **Preserved behaviour (V-ERR-09, P5-deferred):** the child's
//! `ExitStatus` is **discarded** — a non-zero exit does not surface an
//! error, and `run` returns `Ok(())` after `wait()` regardless of the exit
//! status, matching the current `start` semantics. Only `wait`'s I/O
//! `Error` maps to `RunError::Other`. Surfacing the exit status is a later
//! P6 feature.

use std::process::{Command as SystemCommand, Stdio};

use crate::domain::queue::QueueEntry;
use crate::ports::command_runner::{CommandRunner, RunError};

/// Runs queued commands via `fish -c`, inheriting stdio.
pub struct FishRunner {
    /// Shell binary to spawn. Always `fish` in the production wiring; the
    /// `#[cfg(test)]` constructor substitutes a name so the spawn-failure
    /// path can be exercised deterministically (TST-01).
    command: String,
}

impl FishRunner {
    /// A runner that spawns `fish -c <entry>` with inherited stdio.
    pub fn new() -> Self {
        Self {
            command: "fish".to_owned(),
        }
    }

    /// Test seam: spawn an arbitrary binary instead of `fish`, so
    /// `RunError::Spawn` can be exercised with an impossible name no
    /// matter whether `fish` is on the testing machine's PATH.
    #[cfg(test)]
    pub(crate) fn with_command(command: &str) -> Self {
        Self {
            command: command.to_owned(),
        }
    }
}

impl CommandRunner for FishRunner {
    fn run(&self, entry: &QueueEntry) -> Result<(), RunError> {
        let mut child = SystemCommand::new(&self.command)
            .arg("-c")
            .arg(entry.as_str())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(RunError::Spawn)?;

        // V-ERR-09 (P5-deferred): the `ExitStatus` returned by `wait` is
        // deliberately discarded — a non-zero exit is not a failure on this
        // path. Only wait I/O errors map to the port's catch-all variant.
        child.wait().map(|_status| ()).map_err(RunError::Other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether `fish` is resolvable on this machine's `PATH` (TST-01).
    fn fish_on_path() -> bool {
        std::env::var_os("PATH")
            .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join("fish").is_file()))
            .unwrap_or(false)
    }

    #[test]
    fn non_zero_exit_does_not_surface_error() {
        // V-ERR-09 preserved: `fish -c "exit 3"` exits non-zero, yet `run`
        // must return `Ok(())` — the child's `ExitStatus` is discarded,
        // matching the current `start` semantics.
        //
        // Deferral: `fish` is not on this container's PATH, so the
        // non-zero exit cannot be observed here; document and return
        // early instead of failing the suite over a missing interpreter.
        if !fish_on_path() {
            eprintln!(
                "note: `fish` not on PATH; non-zero-exit deferral test is a no-op on this machine"
            );
            return;
        }

        let runner = FishRunner::new();
        let entry = QueueEntry::new("exit 3").unwrap();
        assert!(runner.run(&entry).is_ok());
    }

    #[test]
    fn spawn_failure_maps_to_run_error_spawn() {
        // An impossible binary name makes `spawn` fail deterministically
        // whether or not `fish` is installed, so the `RunError::Spawn`
        // translation is verified either way.
        let runner = FishRunner::with_command("encode-queue-definitely-missing-binary-for-tests");
        let entry = QueueEntry::new("true").unwrap();

        let err = runner
            .run(&entry)
            .expect_err("spawn must fail for an impossible binary");
        assert!(matches!(err, RunError::Spawn(_)));
    }
}
