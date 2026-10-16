//! `QueueLock` port: the exclusive lock guarding the queue against
//! concurrent `start` (P6.F4).
//!
//! The trait signature is final: it returns only `LockError` (ARCH-03).
//! The port references the lock only through this trait — the lock file
//! itself is an outbound concern (ARCH-03, ERR-02).

/// Errors surfaced at the queue-lock boundary.
///
/// One enum beside the port (ERR-02); every expected failure is a named
/// variant with an owned payload (ERR-04, GEN-03).
#[derive(Debug, thiserror::Error)]
pub enum LockError {
    /// Another live process holds the lock.
    #[error("queue already running (pid {0})")]
    HeldBy(u32),
    /// Lock-file I/O failed (create/read/remove).
    #[error("lock file I/O failed: {0}")]
    Io(#[source] std::io::Error),
}

/// An exclusive lock guarding the queue against concurrent `start`,
/// owned by the file lock in `outbound/`.
pub trait QueueLock {
    /// Acquire the lock, reclaiming a stale one.
    fn acquire(&self) -> Result<(), LockError>;
    /// Release the lock (remove the lock file). Idempotent.
    fn release(&self) -> Result<(), LockError>;
}

#[cfg(test)]
mod tests {
    use super::LockError;

    #[test]
    fn variants_carry_owned_payloads() {
        assert!(matches!(LockError::HeldBy(7), LockError::HeldBy(7)));
        assert!(matches!(
            LockError::Io(std::io::Error::other("lock I/O failed")),
            LockError::Io(_)
        ));
    }
}
