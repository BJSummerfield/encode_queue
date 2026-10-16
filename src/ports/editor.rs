//! `EditorLauncher` port: open the queue file in an editor.
//!
//! The trait signature is final: it takes only the `QueuePath` newtype and
//! returns only `EditorError` (ARCH-03).

use crate::domain::queue::QueuePath;

/// Errors surfaced at the editor boundary.
///
/// One enum beside the port (ERR-02); every expected failure is a named
/// variant with an owned payload plus the `Other` catch-all (ERR-04, GEN-03).
#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    /// Spawning the editor process failed (e.g. the `$EDITOR` binary is
    /// missing).
    #[error("failed to spawn the editor: {0}")]
    Spawn(#[source] std::io::Error),
    /// Any other unclassifiable launcher failure.
    #[error("the editor failed to launch: {0}")]
    Other(std::io::Error),
}

/// Launches an editor on the queue file.
pub trait EditorLauncher {
    /// Open the queue located at `at` in the configured editor.
    fn open(&self, at: &QueuePath) -> Result<(), EditorError>;
}

#[cfg(test)]
mod tests {
    use super::EditorError;
    use std::io::{Error, ErrorKind};

    #[test]
    fn variants_carry_owned_payloads() {
        assert!(matches!(
            EditorError::Spawn(Error::new(ErrorKind::NotFound, "no nvim")),
            EditorError::Spawn(_)
        ));
        assert!(matches!(
            EditorError::Other(std::io::Error::other("editor failed")),
            EditorError::Other(_)
        ));
    }
}
