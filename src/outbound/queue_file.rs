//! `FileQueueStore` outbound adapter (the queue file boundary).
//!
//! WP04: implements the `QueueStore` port over the queue file. The store
//! owns its `QueuePath`, so no raw `PathBuf` crosses the port, and all
//! `OpenOptions`/`std::fs` details stay confined to this file (ARCH-03).
//!
//! Error mapping (V-ERR-08): a missing file is the named
//! `QueueStoreError::QueueFileMissing` variant (→ exit 1 at `main`, the
//! preserved `ls`-on-missing-file outcome; mapping it to an empty queue is
//! the deferred P6 change), and any other I/O failure is the `Io` catch-all
//! (ERR-04). `read_all` parses each line through `QueueEntry`'s lenient
//! `FromStr`: every line, including blank/whitespace-only ones, is a valid
//! entry kept verbatim, so `ls` prints them unchanged (behaviour
//! preservation; the strict reject is deferred).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::str::FromStr;

use crate::domain::queue::{QueueEntry, QueuePath};
use crate::ports::queue_store::{QueueStore, QueueStoreError};

/// File-backed [`QueueStore`] owning its queue file path.
pub struct FileQueueStore {
    path: QueuePath,
}

impl FileQueueStore {
    /// Wrap the queue file path; the store owns it from here on.
    pub fn new(path: QueuePath) -> Self {
        Self { path }
    }
}

/// Classify an adapter I/O error: a missing file is
/// `QueueFileMissing` (V-ERR-08), everything else the `Io` catch-all
/// (ERR-04).
fn map_io_error(path: &Path, err: std::io::Error) -> QueueStoreError {
    if err.kind() == std::io::ErrorKind::NotFound {
        QueueStoreError::QueueFileMissing(path.to_path_buf())
    } else {
        QueueStoreError::Io(err)
    }
}

impl QueueStore for FileQueueStore {
    fn read_all(&self) -> Result<Vec<QueueEntry>, QueueStoreError> {
        let contents = match fs::read_to_string(self.path.as_path()) {
            Ok(contents) => contents,
            Err(err) => return Err(map_io_error(self.path.as_path(), err)),
        };
        // Every line — blank and whitespace-only included — parses through
        // the lenient `FromStr` and is kept verbatim. The `Err` arm is
        // unreachable on the P5 (lenient) path but is mapped to the catch-all
        // rather than panicked on (GEN-01).
        contents
            .lines()
            .map(QueueEntry::from_str)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| QueueStoreError::Io(std::io::Error::other("queue entry failed to parse")))
    }

    fn append(&self, entry: &QueueEntry) -> Result<(), QueueStoreError> {
        let mut file = match OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.path.as_path())
        {
            Ok(file) => file,
            Err(err) => return Err(map_io_error(self.path.as_path(), err)),
        };
        writeln!(file, "{}", entry).map_err(QueueStoreError::Io)
    }

    fn drop_first(&self) -> Result<(), QueueStoreError> {
        let entries = self.read_all()?;
        if entries.is_empty() {
            // Preserved behaviour: the `start` drain terminates on an empty
            // queue without rewriting the (already empty) file.
            return Ok(());
        }
        let mut file = match OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(self.path.as_path())
        {
            Ok(file) => file,
            Err(err) => return Err(map_io_error(self.path.as_path(), err)),
        };
        for entry in &entries[1..] {
            writeln!(file, "{}", entry).map_err(QueueStoreError::Io)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    // Each test gets a unique temp subdirectory under `std::env::temp_dir()`;
    // no test touches `$HOME/.config/encode_queue` (TST-01).
    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn temp_queue_path() -> PathBuf {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("encode_queue_wp04_{}_{id}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir.join(format!("commands_{id}.txt"))
    }

    fn store(path: &Path) -> FileQueueStore {
        FileQueueStore::new(QueuePath::new(path.to_path_buf()))
    }

    #[test]
    fn read_all_on_missing_file_is_queue_file_missing() {
        let path = temp_queue_path();
        assert!(!path.exists());

        let err = store(&path).read_all().unwrap_err();
        assert!(matches!(
            err,
            QueueStoreError::QueueFileMissing(missing) if missing == path
        ));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn read_all_preserves_blank_and_whitespace_lines_verbatim() {
        let path = temp_queue_path();
        fs::write(&path, "nix build .\n   \n\tdent\n\n").unwrap();

        let entries = store(&path).read_all().unwrap();
        let lines: Vec<&str> = entries.iter().map(QueueEntry::as_str).collect();
        assert_eq!(lines, vec!["nix build .", "   ", "\tdent", ""]);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn append_then_read_all_round_trip() {
        let path = temp_queue_path();
        let queue = store(&path);

        queue
            .append(&QueueEntry::from_str("nix build .").unwrap())
            .unwrap();
        queue.append(&QueueEntry::from_str("   ").unwrap()).unwrap();
        queue
            .append(&QueueEntry::from_str("cargo test").unwrap())
            .unwrap();

        let entries = queue.read_all().unwrap();
        let lines: Vec<&str> = entries.iter().map(QueueEntry::as_str).collect();
        assert_eq!(lines, vec!["nix build .", "   ", "cargo test"]);

        // The blank line is stored verbatim, not trimmed or omitted.
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "nix build .\n   \ncargo test\n"
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn drop_first_leaves_exactly_the_remaining_lines() {
        let path = temp_queue_path();
        fs::write(&path, "nix build .\n   \ncargo test\n").unwrap();
        let queue = store(&path);

        queue.drop_first().unwrap();
        let entries = queue.read_all().unwrap();
        let lines: Vec<&str> = entries.iter().map(QueueEntry::as_str).collect();
        assert_eq!(lines, vec!["   ", "cargo test"]);

        queue.drop_first().unwrap();
        let entries = queue.read_all().unwrap();
        let lines: Vec<&str> = entries.iter().map(QueueEntry::as_str).collect();
        assert_eq!(lines, vec!["cargo test"]);

        queue.drop_first().unwrap();
        assert!(queue.read_all().unwrap().is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), "");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn drop_first_on_empty_queue_is_ok_without_rewriting() {
        let path = temp_queue_path();
        fs::write(&path, "").unwrap();

        store(&path).drop_first().unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "");

        let _ = fs::remove_file(&path);
    }
}
