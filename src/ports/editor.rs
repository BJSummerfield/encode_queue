//! `EditorLauncher` port: open the queue file in an editor.
//!
//! Scaffold (WP01): the trait signature is final; `EditorError` is a
//! `thiserror` skeleton (full variant set in WP03).

use crate::domain::queue::QueuePath;

/// Errors surfaced at the editor boundary.
///
/// Skeleton; the concrete variants (`Spawn`, `Other`) land in WP03.
#[derive(Debug, thiserror::Error)]
pub enum EditorError {}

/// Launches an editor on the queue file.
pub trait EditorLauncher {
    /// Open the queue located at `at` in the configured editor.
    fn open(&self, at: &QueuePath) -> Result<(), EditorError>;
}
