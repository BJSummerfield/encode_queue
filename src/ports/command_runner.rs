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
    /// Spawning the `fish -c` child failed.
    #[error("failed to spawn `fish -c`: {0}")]
    Spawn(#[source] std::io::Error),
    /// The child exited with a non-zero exit status.
    ///
    /// Declared as part of the frozen boundary; the current path discards the
    /// child's exit status (V-ERR-09 is a deferred P5 behaviour change), so no
    /// adapter constructs this variant yet.
    // Deferred landing point (V-ERR-09 exit-status surfacing is a P5/P6 change per PLAN.md); not constructed before then, hence the non-test dead_code suppression.
    #[cfg_attr(not(test), allow(dead_code))]
    #[error("`fish -c` exited unsuccessfully: {code}")]
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
