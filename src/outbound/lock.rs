//! `FileQueueLock` outbound adapter (the queue-lock boundary, P6.F4).
//!
//! Implements the `QueueLock` port over a lock file: `acquire` creates the
//! file with `create_new` and writes this process's PID into it; `release`
//! removes it. All `std::fs`/`std::process` details stay confined to this
//! file (ARCH-03); every I/O failure maps into the port's `LockError`
//! (ERR-04). The lock guards only `start`: `add`/`ls`/`rm`/`edit` never
//! touch it.
//!
//! Staleness check: a held lock whose recorded PID is not alive is
//! reclaimed. `pid_is_alive` is **Linux-only** (it checks `/proc/<pid>`):
//! on non-Linux targets every PID reads as dead, so locks are always
//! reclaimed — acceptable for this tool (single-user, Linux devbox).
//! An unreadable or garbage lock file is always treated as stale and
//! reclaimed: for a manual queue tool the safe direction is to reclaim.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::domain::queue::QueuePath;
use crate::ports::queue_lock::{LockError, QueueLock};

/// The lock file name, living ONLY in this adapter (acceptance grep): every
/// other layer talks about "the lock" through the port without naming the
/// file.
const LOCK_FILE: &str = "queue.lock";

/// A bound on the acquisition retry loop: a held lock whose stale removal
/// loses a race is retried a bounded number of times before the adapter
/// gives up (ERR-04: no infinite loops).
const ACQUIRE_ATTEMPTS: u32 = 3;

/// File-backed [`QueueLock`] owning its lock-file path.
pub struct FileQueueLock {
    lock_path: PathBuf,
}

impl FileQueueLock {
    /// Wrap the queue file path; the lock file lives at
    /// `<parent of queue_path>/queue.lock` — the same
    /// parent-of-queue-path convention as the failure log (a missing parent
    /// maps to the current directory; no `unwrap`, GEN-01).
    pub fn new(queue_path: QueuePath) -> Self {
        Self {
            lock_path: lock_path_of(queue_path.as_path()),
        }
    }
}

/// The lock-file location for a queue file: the queue file's parent
/// directory joined with the lock file name. A queue path with no parent
/// (a root-only path) or an empty parent (a bare file name) maps the lock
/// to the process's current directory via the relative name — no `unwrap`,
/// no `env::var` (GEN-01).
fn lock_path_of(queue_path: &Path) -> PathBuf {
    match queue_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(LOCK_FILE),
        _ => PathBuf::from(LOCK_FILE),
    }
}

impl QueueLock for FileQueueLock {
    /// Acquire the lock: create the lock file with `create_new` and write
    /// this process's PID into it (decimal digits, no trailing newline —
    /// the reader trims anyway). On `AlreadyExists`, read the recorded
    /// holder PID: a live holder is refused with `HeldBy`, a dead (or
    /// unreadable/garbage) holder is removed and the retry loop retries.
    /// Bounded to `ACQUIRE_ATTEMPTS`; the final failure is the `Io`
    /// catch-all (ERR-04).
    fn acquire(&self) -> Result<(), LockError> {
        for _ in 0..ACQUIRE_ATTEMPTS {
            match File::create_new(&self.lock_path) {
                Ok(mut file) => {
                    // `std::process` detail: our own PID, written as
                    // decimal digits without a trailing newline.
                    let pid = std::process::id();
                    file.write_all(pid.to_string().as_bytes())
                        .map_err(LockError::Io)?;
                    return Ok(());
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    match read_holder_pid(&self.lock_path) {
                        // A live holder keeps the lock; refuse.
                        Some(pid) if pid_is_alive(pid) => {
                            return Err(LockError::HeldBy(pid));
                        }
                        // Stale or unreadable: remove the lock and fall
                        // through to the retry.
                        _ => {
                            let _ = fs::remove_file(&self.lock_path);
                        }
                    }
                }
                // Any other I/O failure is the catch-all (ERR-04).
                Err(e) => return Err(LockError::Io(e)),
            }
        }
        Err(LockError::Io(std::io::Error::other(
            "could not acquire the queue lock after 3 attempts",
        )))
    }

    /// Release the lock: remove the lock file. Idempotent — a missing file
    /// is `Ok(())`; any other I/O failure is the `Io` catch-all.
    fn release(&self) -> Result<(), LockError> {
        match fs::remove_file(&self.lock_path) {
            Ok(()) => Ok(()),
            // Idempotent: the lock is already gone.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(LockError::Io(e)),
        }
    }
}

/// Read the PID recorded in the lock file: trim whitespace, parse as `u32`.
/// Any I/O or parse failure maps to `None` — an unreadable or garbage lock
/// is treated as stale and is always reclaimable, which is the safe
/// direction for a manual queue tool.
fn read_holder_pid(path: &Path) -> Option<u32> {
    match fs::read_to_string(path) {
        Ok(contents) => contents.trim().parse::<u32>().ok(),
        Err(_) => None,
    }
}

/// Whether `pid` is alive, **Linux-only**: `pid != 0` and `/proc/<pid>`
/// exists. On non-Linux targets every PID reads as dead (see the module
/// doc).
fn pid_is_alive(pid: u32) -> bool {
    pid != 0 && Path::new("/proc").join(pid.to_string()).exists()
}

#[cfg(test)]
mod tests {
    use super::{lock_path_of, pid_is_alive, FileQueueLock};
    use crate::ports::queue_lock::QueueLock;
    use std::fs;
    use std::path::PathBuf;

    /// Probe downward from a PID far above any realistic `pid_max` for one
    /// with no `/proc/<pid>` entry (fits `u32`; test-only `panic!`, TST-03).
    fn find_dead_pid() -> u32 {
        let mut pid: u32 = 4_000_000_000;
        while pid_is_alive(pid) {
            pid -= 1;
        }
        pid
    }

    /// A fresh temp dir per test; no test touches `$HOME/.config/encode_queue`
    /// (TST-01). The `QueuePath` points at a (possibly absent) queue file
    /// inside the dir, so the lock lands next to it. `keep()` consumes the
    /// handle, disables drop-based cleanup and yields the path; each test
    /// removes the dir manually (the deprecated `into_path` is not used).
    fn temp_lock() -> (std::path::PathBuf, FileQueueLock) {
        let dir = tempfile::tempdir().expect("tempdir available");
        let queue_path = dir.path().join("commands.txt");
        let path = dir.keep();
        (
            path,
            FileQueueLock::new(crate::domain::queue::QueuePath::new(queue_path)),
        )
    }

    #[test]
    fn acquire_creates_the_lock_file_containing_exactly_our_pid() {
        let (dir, lock) = temp_lock();
        let lock_file = dir.join("queue.lock");

        lock.acquire().expect("acquire on a free lock succeeds");

        // Exactly our PID: decimal digits, no trailing newline.
        assert_eq!(
            fs::read(&lock_file).expect("lock file readable"),
            std::process::id().to_string().as_bytes()
        );

        lock.release().expect("release succeeds");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_second_lock_on_the_same_path_is_held_by_us() {
        let (dir, first) = temp_lock();
        let second = FileQueueLock::new(crate::domain::queue::QueuePath::new(
            dir.join("commands.txt"),
        ));

        first.acquire().expect("first acquire succeeds");
        let our_pid = std::process::id();
        let err = second.acquire().expect_err("second acquire must fail");
        assert!(
            matches!(err, crate::ports::queue_lock::LockError::HeldBy(p) if p == our_pid),
            "expected HeldBy(our pid), got {err:?}"
        );
        // The first holder's lock file is untouched.
        let lock_file = dir.join("queue.lock");
        assert_eq!(
            fs::read(&lock_file).expect("lock file readable"),
            our_pid.to_string().as_bytes()
        );

        first.release().expect("release succeeds");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_stale_lock_with_a_dead_pid_is_reclaimed() {
        let (dir, lock) = temp_lock();
        let lock_file = dir.join("queue.lock");
        fs::write(&lock_file, find_dead_pid().to_string()).expect("stale lock written");

        lock.acquire().expect("acquire reclaims a stale lock");

        assert_eq!(
            fs::read(&lock_file).expect("lock file readable"),
            std::process::id().to_string().as_bytes()
        );

        lock.release().expect("release succeeds");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_lock_with_garbage_contents_is_reclaimed() {
        let (dir, lock) = temp_lock();
        let lock_file = dir.join("queue.lock");
        fs::write(&lock_file, b"not a pid\n").expect("garbage lock written");

        lock.acquire().expect("acquire reclaims a garbage lock");

        assert_eq!(
            fs::read(&lock_file).expect("lock file readable"),
            std::process::id().to_string().as_bytes()
        );

        lock.release().expect("release succeeds");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn release_removes_the_lock_and_is_idempotent_when_absent() {
        let (dir, lock) = temp_lock();
        let lock_file = dir.join("queue.lock");

        lock.acquire().expect("acquire succeeds");
        assert!(lock_file.exists());
        lock.release().expect("release succeeds");
        assert!(!lock_file.exists());
        // Idempotent: releasing again (file already gone) is `Ok`.
        lock.release().expect("release is idempotent");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn pid_is_alive_checks_zero_and_self() {
        assert!(!pid_is_alive(0));
        assert!(pid_is_alive(std::process::id()));
    }

    #[test]
    fn lock_path_lives_next_to_the_queue_file() {
        let queue = PathBuf::from("/home/user/.config/encode_queue/commands.txt");
        assert_eq!(
            lock_path_of(&queue),
            PathBuf::from("/home/user/.config/encode_queue/queue.lock")
        );

        // A bare file name has an empty parent: the lock lands in the
        // current directory (the relative name).
        let bare = PathBuf::from("commands.txt");
        assert_eq!(lock_path_of(&bare), PathBuf::from("queue.lock"));
        // A root-only path has no parent: same current-directory mapping.
        let root = PathBuf::from("/");
        assert_eq!(lock_path_of(&root), PathBuf::from("queue.lock"));
    }
}
