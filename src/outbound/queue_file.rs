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
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::domain::failure::FailureRecord;
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

/// The failure-log file name, living ONLY in this adapter (acceptance
/// grep): every other layer talks about "the failure log" without naming
/// the file.
const FAILURE_LOG_FILE: &str = "failed.txt";

/// The directory holding the failure log: the parent directory of the
/// stored queue path — the same convention the store already uses for the
/// queue file itself. A queue path with no parent maps to the current
/// directory (no `unwrap`; GEN-01).
fn queue_dir(path: &Path) -> Result<PathBuf, QueueStoreError> {
    match path.parent() {
        Some(parent) => Ok(parent.to_path_buf()),
        None => Ok(std::env::current_dir().map_err(QueueStoreError::Io)?),
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

    /// Replace the entire queue with `entries` (truncate + rewrite).
    ///
    /// Same truncate-open + `map_io_error` + per-entry `writeln!` as
    /// `drop_first`, with one deliberate difference: `write_all` rewrites
    /// the file **even for an empty `entries` slice** (the truncate+rewrite
    /// runs unconditionally, leaving an empty file), whereas `drop_first`
    /// skips the rewrite on an empty queue.
    fn write_all(&self, entries: &[QueueEntry]) -> Result<(), QueueStoreError> {
        let mut file = match OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(self.path.as_path())
        {
            Ok(file) => file,
            Err(err) => return Err(map_io_error(self.path.as_path(), err)),
        };
        for entry in entries {
            writeln!(file, "{}", entry).map_err(QueueStoreError::Io)?;
        }
        Ok(())
    }

    /// Append one failure record to the failure log in the queue directory:
    /// open the log with `create(true).append(true)` (append semantics, same
    /// as `append`) and write exactly one
    /// `<unix_ts>\t<exit_code>\t<command>\n` line (P6.F3).
    ///
    /// The queue directory is the parent of the stored queue path (a queue
    /// path with no parent maps to the current directory); the queue file
    /// itself is never touched. A missing parent directory surfaces as
    /// `QueueFileMissing` via `map_io_error`; any other I/O failure is the
    /// `Io` catch-all (ERR-04).
    fn append_failure(&self, record: &FailureRecord) -> Result<(), QueueStoreError> {
        let dir = queue_dir(self.path.as_path())?;
        let path = dir.join(FAILURE_LOG_FILE);
        let mut file = match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(file) => file,
            Err(err) => return Err(map_io_error(&path, err)),
        };
        writeln!(
            file,
            "{}\t{}\t{}",
            record.timestamp(),
            record.exit_code(),
            record.command().as_str()
        )
        .map_err(QueueStoreError::Io)
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

    fn failure(id: u64, exit_code: i32, command: &str) -> FailureRecord {
        FailureRecord::new(id, exit_code, QueueEntry::from_str(command).unwrap())
    }

    /// P6.F3: one `append_failure` creates the failure log in the queue
    /// directory (the parent of the queue path) with exactly one
    /// `<unix_ts>\t<exit_code>\t<command>\n` line — asserted as raw bytes for
    /// a known record.
    #[test]
    fn append_failure_creates_failure_log_with_exact_line_format() {
        let path = temp_queue_path();
        let queue = store(&path);
        let log = path.parent().unwrap().join("failed.txt");
        assert!(!log.exists());

        queue
            .append_failure(&failure(1_700_000_000, 3, "false"))
            .unwrap();

        assert_eq!(fs::read(&log).unwrap(), b"1700000000\t3\tfalse\n");

        let _ = fs::remove_file(&log);
    }

    /// P6.F3: two appends both present, in call order (append semantics,
    /// never truncate).
    #[test]
    fn append_failure_twice_keeps_both_lines_in_order() {
        let path = temp_queue_path();
        let queue = store(&path);
        let log = path.parent().unwrap().join("failed.txt");

        queue
            .append_failure(&failure(1_700_000_000, 1, "false"))
            .unwrap();
        queue
            .append_failure(&failure(1_700_000_001, 2, "exit 2"))
            .unwrap();

        assert_eq!(
            fs::read(&log).unwrap(),
            b"1700000000\t1\tfalse\n1700000001\t2\texit 2\n"
        );

        let _ = fs::remove_file(&log);
    }

    /// P6.F3: `append_failure` never touches the queue file itself — its
    /// bytes are identical before and after, and the queue's `read_all`
    /// still sees exactly the original entries.
    #[test]
    fn append_failure_leaves_the_queue_file_untouched() {
        let path = temp_queue_path();
        fs::write(&path, "one\ntwo\n").unwrap();
        let queue = store(&path);

        queue
            .append_failure(&failure(1_700_000_000, 1, "one"))
            .unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "one\ntwo\n");
        let entries = queue.read_all().unwrap();
        let lines: Vec<&str> = entries.iter().map(QueueEntry::as_str).collect();
        assert_eq!(lines, vec!["one", "two"]);

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.parent().unwrap().join("failed.txt"));
    }

    /// P6.F3: a queue path whose parent directory does not exist cannot be
    /// appended to: opening the failure log fails `NotFound` and is
    /// classified as `QueueFileMissing` carrying the (missing) queue
    /// directory (ERR-04: no new error variant).
    #[test]
    fn append_failure_in_missing_parent_directory_is_queue_file_missing() {
        let path = temp_queue_path();
        let queue_dir = path.parent().unwrap().to_path_buf();
        fs::remove_dir(&queue_dir).unwrap(); // the queue path's parent is gone
        let queue = store(&path);

        let err = queue.append_failure(&failure(1, 1, "cmd")).unwrap_err();
        // `map_io_error` classifies against the path it tried to open —
        // the log file inside the missing queue directory.
        assert!(matches!(
            err,
            QueueStoreError::QueueFileMissing(missing) if missing == queue_dir.join("failed.txt")
        ));
    }
}
