//! `CommandRunner` port: run a single queued command.
//!
//! The trait signature is final: it takes only domain newtypes and returns
//! only `RunError` (ARCH-03).

use crate::domain::queue::QueueEntry;

/// Errors surfaced at the command-runner boundary.
///
/// One enum beside the port (ERR-02); every expected failure is a named
/// variant with an owned payload plus the `Other` catch-all (ERR-04, GEN-03).
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// Spawning the shell child failed.
    #[error("failed to spawn the shell: {0}")]
    Spawn(#[source] std::io::Error),
    /// The shell command exited with a non-zero exit status.
    ///
    /// Surfaced by the runner (V-ERR-09 runner half, P6.F2): a non-zero
    /// exit after `wait()` is a failure. The service currently propagates it
    /// unchanged (P6.F3 owns the failure-handling behaviour change).
    #[error("shell command exited unsuccessfully: {code}")]
    Waited { code: i32 },
    /// Wait I/O failed, or any other unclassifiable runner failure.
    #[error("running the command failed: {0}")]
    Other(std::io::Error),
}

/// Runs queued commands.
pub trait CommandRunner {
    /// Run a single entry, inheriting stdio.
    fn run(&self, entry: &QueueEntry) -> Result<(), RunError>;
}

#[cfg(test)]
mod tests {
    use super::RunError;

    #[test]
    fn variants_carry_owned_payloads() {
        assert!(matches!(
            RunError::Spawn(std::io::Error::other("no fish")),
            RunError::Spawn(_)
        ));
        assert!(matches!(
            RunError::Waited { code: 3 },
            RunError::Waited { code: 3 }
        ));
        assert!(matches!(
            RunError::Other(std::io::Error::other("wait failed")),
            RunError::Other(_)
        ));
    }
}
