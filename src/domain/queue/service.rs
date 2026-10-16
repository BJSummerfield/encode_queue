//! Application/service layer for the queue.
//!
//! WP08: `QueueService` is generic over its four injected ports
//! (`R: QueueStore`, `C: CommandRunner`, `E: EditorLauncher`,
//! `L: QueueLock`) and exposes `add`/`ls`/`start`/`edit`. Every method
//! takes and returns only domain newtypes and the [`QueueError`] enum
//! (ARCH-04): no `&str`/`String` parameters, and the module imports only
//! `crate::domain` and `crate::ports` (ARCH-01).
//! `QueueError` composes the five boundary enums via `#[from]` and never
//! duplicates their variants (ERR-02).
//!
//! A fourth port generic (`L: QueueLock`) beyond the `QueueService<R, C>`
//! shorthand is intentional: it lets all four ports be injected with mocks
//! (TST-01), which the three-generic form cannot express.
//!
//! `start` preserves the legacy `process_queue` flow — trim-before-run,
//! unconditional removal of the first line (blank lines are removed, not
//! run), a fresh re-read + rewrite each iteration, and immediate error
//! propagation before the rewrite — with the P6.F3 failure handling added:
//! a non-zero exit status (`RunError::Waited`, V-ERR-09 runner half, P6.F2)
//! logs a `FailureRecord` to the store's failure log and removes the failed
//! entry, then `start` either continues (the default) or stops with
//! `QueueError::StoppedOnFailure` (`--stop-on-error`). Both modes remove the
//! failed entry; the flag only changes stop-vs-continue and the exit status.
//! Spawn/other runner failures mean the command never ran: nothing is
//! logged, the entry stays queued, and the error propagates unchanged.
//! The strict trim + `Empty`-reject (V-TY-02) is a deferred P6 change and
//! is NOT implemented here.

use crate::domain::failure::FailureRecord;
use crate::domain::queue::{QueueEntry, QueueEntryError, QueueEntryIndex, QueuePath};
use crate::ports::command_runner::{CommandRunner, RunError};
use crate::ports::editor::{EditorError, EditorLauncher};
use crate::ports::queue_lock::{LockError, QueueLock};
use crate::ports::queue_store::{QueueStore, QueueStoreError};
use std::time::{SystemTime, UNIX_EPOCH};

/// The queue service, generic over its four injected ports.
///
/// A fourth port generic (`L: QueueLock`) beyond the `QueueService<R, C>`
/// shorthand is required so that all four ports can be injected with mocks
/// (TST-01). The queue `QueuePath` is deliberately NOT a field here: `Config`
/// remains the single origin of that path (V-ARCH-05), so the service depends
/// only on the ports (ARCH-01).
#[derive(Debug, Clone)]
pub struct QueueService<R, C, E, L>
where
    R: QueueStore,
    C: CommandRunner,
    E: EditorLauncher,
    L: QueueLock,
{
    store: R,
    runner: C,
    editor: E,
    lock: L,
}

/// Errors surfaced by the queue service.
///
/// Composes the five boundary enums via `#[from]` and never duplicates their
/// variants (ERR-02): `Store` for queue-file I/O, `Run` for command execution,
/// `Editor` for editor launch, `Lock` for the start lock (P6.F4), and
/// `Invalid` for a line rejected by the domain's `QueueEntry` constructor.
#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    /// The queue store (file I/O boundary) failed.
    #[error(transparent)]
    Store(#[from] QueueStoreError),
    /// Running a queued command failed at the runner boundary.
    #[error(transparent)]
    Run(#[from] RunError),
    /// Launching the editor failed at the editor boundary.
    #[error(transparent)]
    Editor(#[from] EditorError),
    /// A queued line was rejected by the domain `QueueEntry` constructor.
    #[error("command rejected by the domain: {0}")]
    Invalid(#[from] QueueEntryError),
    /// `remove_entry` was asked for a position with no entry (0 or beyond the
    /// queue's length). The queue is left untouched.
    #[error("no queue entry at position {0}")]
    NoEntry(QueueEntryIndex),
    /// `--stop-on-error` was set and a command failed.
    #[error("stopped after a failed command (--stop-on-error)")]
    StoppedOnFailure,
    /// The start lock could not be acquired or released.
    #[error(transparent)]
    Lock(#[from] LockError),
}

impl<R, C, E, L> QueueService<R, C, E, L>
where
    R: QueueStore,
    C: CommandRunner,
    E: EditorLauncher,
    L: QueueLock,
{
    /// Wire the four ports into a service.
    pub fn new(store: R, runner: C, editor: E, lock: L) -> Self {
        Self {
            store,
            runner,
            editor,
            lock,
        }
    }

    /// Append a single entry to the queue.
    pub fn add(&self, entry: &QueueEntry) -> Result<(), QueueError> {
        self.store.append(entry)?;
        Ok(())
    }

    /// List every entry in the queue, in order (blank lines verbatim).
    ///
    /// P6 (V-ARCH-05 behaviour half): a **missing** queue file is not an
    /// error for `ls` — it is reported as an empty queue (`Ok(Vec::new())`),
    /// while any other store error still propagates. `start` is deliberately
    /// UNCHANGED: a missing queue still surfaces there as
    /// `QueueError::Store(QueueStoreError::QueueFileMissing)`.
    pub fn ls(&self) -> Result<Vec<QueueEntry>, QueueError> {
        match self.store.read_all() {
            Err(QueueStoreError::QueueFileMissing(_)) => Ok(Vec::new()),
            Err(e) => Err(e.into()),
            Ok(entries) => Ok(entries),
        }
    }

    /// Run the queued entries until the queue is empty.
    ///
    /// P6.F4: the start lock guards the whole drain — it is acquired before
    /// the first read, and the release runs on every path. A release failure
    /// after a successful drain surfaces as `QueueError::Lock`; when the
    /// drain itself failed, the drain error wins (the release error is
    /// discarded). A held lock is surfaced as
    /// `QueueError::Lock(LockError::HeldBy(pid))` before any store access.
    pub fn start(&self, stop_on_error: bool) -> Result<(), QueueError> {
        self.lock.acquire()?;
        let result = self.drain(stop_on_error);
        let release = self.lock.release();
        match (result, release) {
            (Ok(()), Ok(())) => Ok(()),
            // The drain error wins.
            (Err(e), _) => Err(e),
            // Failed to release after a successful drain.
            (Ok(()), Err(e)) => Err(e.into()),
        }
    }

    /// The `start` drain loop: run the queued entries, one per iteration,
    /// until the queue is empty.
    ///
    /// Each iteration reads the whole queue (a missing file surfaces as
    /// `QueueError::Store`), breaks on an empty queue without rewriting, trims
    /// the first line and runs it only when non-blank (a blank first line is
    /// not run but is still removed — the removal is unconditional), then drops
    /// the first entry by re-reading and rewriting the remainder. A store or
    /// runner error propagates immediately, before the rewrite, leaving the
    /// queue unmodified up to that point.
    ///
    /// P6.F3 failure handling: when the runner reports a non-zero exit
    /// (`RunError::Waited { code }`), the failure is logged to the store's
    /// failure log as a [`FailureRecord`] (with the observed Unix timestamp
    /// and the trimmed command) and the failed entry is removed, after which
    /// `start` either continues with the next entry (the default) or stops
    /// with `QueueError::StoppedOnFailure` when `stop_on_error` is set. Both
    /// modes remove the failed entry; the flag only changes stop-vs-continue
    /// and the exit status. Spawn/other runner failures are never logged:
    /// the command never ran, so the entry stays queued and the error
    /// propagates unchanged (exit 1 at the boundary).
    fn drain(&self, stop_on_error: bool) -> Result<(), QueueError> {
        loop {
            // Fresh read each iteration: the queue "may have been edited"
            // between iterations.
            let entries = self.store.read_all()?;

            // Legacy `read_line() == 0` → break; no rewrite on an empty queue.
            if entries.is_empty() {
                break;
            }

            let first = &entries[0];
            if !first.as_str().trim().is_empty() {
                // Trim before running. `QueueEntry::new` is lenient on the P5
                // path (never fails); its fallible `Err` arm maps through
                // `#[from]` to `QueueError::Invalid` (GEN-01: no `unwrap`).
                let trimmed = QueueEntry::new(first.as_str().trim())?;
                // P6.F3: a non-zero exit (`RunError::Waited`) logs the
                // failure, removes the entry, then either continues or stops.
                match self.runner.run(&trimmed) {
                    Ok(()) => {}
                    Err(RunError::Waited { code }) => {
                        // Non-zero exit: log the failure, remove the entry,
                        // then either continue or stop (binding order).
                        self.store.append_failure(&FailureRecord {
                            timestamp: unix_now(),
                            exit_code: code,
                            command: trimmed.clone(),
                        })?;
                        self.store.drop_first()?;
                        if stop_on_error {
                            return Err(QueueError::StoppedOnFailure);
                        }
                        // The failed entry was removed above: skip the
                        // unconditional removal at the end of the loop body
                        // and continue with the next entry.
                        continue;
                    }
                    // Spawn/I/O failure: the command never ran — unchanged
                    // behaviour: no failure-log record, the entry stays
                    // queued, and the error propagates (exit 1 at the
                    // boundary).
                    Err(e) => return Err(e.into()),
                }
            }
            // Unconditional removal: even a blank first line is dropped.
            // `drop_first` re-reads and rewrites the remainder (empty → no
            // rewrite, WP04-pinned).
            self.store.drop_first()?;
        }
        Ok(())
    }

    /// Open the queue located at `at` in the configured editor.
    ///
    /// The path is supplied by the caller (`Config` is the single origin of the
    /// queue `QueuePath`, V-ARCH-05); the service depends only on the editor
    /// port (ARCH-01).
    pub fn edit(&self, at: &QueuePath) -> Result<(), QueueError> {
        self.editor.open(at)?;
        Ok(())
    }

    /// Remove the queue entry at 1-based position `pos`, keeping the rest in
    /// order. `pos == 0` or `pos > <queue length>` is `QueueError::NoEntry(pos)`
    /// and leaves the queue untouched; a missing queue file surfaces as
    /// `QueueError::Store(QueueStoreError::QueueFileMissing)` before any
    /// rewrite (deliberately not softened the way `ls` is).
    ///
    /// One `read_all` + one `write_all` (the store's atomic truncate+rewrite):
    /// the filtered-out entry is the one at 0-based index `pos - 1`. Removing
    /// the only entry rewrites an empty file (`write_all` runs unconditionally
    /// on an empty slice).
    pub fn remove_entry(&self, pos: QueueEntryIndex) -> Result<(), QueueError> {
        let position = pos.as_u32();
        let entries = self.store.read_all()?;
        if position == 0 || position > entries.len() as u32 {
            return Err(QueueError::NoEntry(pos));
        }
        let target = position as usize - 1;
        let filtered: Vec<QueueEntry> = entries
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != target)
            .map(|(_, e)| e.clone())
            .collect();
        self.store.write_all(&filtered)?;
        Ok(())
    }
}

/// The current Unix timestamp in seconds, for `FailureRecord` (P6.F3).
///
/// `SystemTime::now()` is only documented as relative to the Unix epoch, so a
/// pre-epoch (impossible) reading would make `duration_since` fail; that `Err`
/// maps to `0` via `map_or` rather than panicking (GEN-01: no `unwrap`).
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// In-memory `QueueStore` mock: `entries` is the backing queue; `read_err`
    /// and `drop_err` are one-shot scripted failures (returned on the next
    /// matching call). Purely in-memory: no real file, no environment access.
    /// P6.F3: `failures` records every `append_failure` call and
    /// `drop_calls` counts every `drop_first` call, so tests can assert the
    /// failure-log and removal behaviour without touching the filesystem.
    struct MockStore {
        entries: Mutex<Vec<QueueEntry>>,
        read_err: Mutex<Option<QueueStoreError>>,
        drop_err: Mutex<Option<QueueStoreError>>,
        failures: Mutex<Vec<FailureRecord>>,
        drop_calls: Mutex<u64>,
    }

    impl QueueStore for MockStore {
        fn read_all(&self) -> Result<Vec<QueueEntry>, QueueStoreError> {
            if let Some(err) = self.read_err.lock().unwrap().take() {
                return Err(err);
            }
            Ok(self.entries.lock().unwrap().clone())
        }

        fn append(&self, entry: &QueueEntry) -> Result<(), QueueStoreError> {
            self.entries.lock().unwrap().push(entry.clone());
            Ok(())
        }

        fn drop_first(&self) -> Result<(), QueueStoreError> {
            if let Some(err) = self.drop_err.lock().unwrap().take() {
                return Err(err);
            }
            *self.drop_calls.lock().unwrap() += 1;
            let mut entries = self.entries.lock().unwrap();
            if !entries.is_empty() {
                entries.remove(0);
            }
            Ok(())
        }

        fn write_all(&self, entries: &[QueueEntry]) -> Result<(), QueueStoreError> {
            *self.entries.lock().unwrap() = entries.to_vec();
            Ok(())
        }

        fn append_failure(&self, record: &FailureRecord) -> Result<(), QueueStoreError> {
            self.failures.lock().unwrap().push(record.clone());
            Ok(())
        }
    }

    /// `CommandRunner` mock: records each `as_str()` it was asked to run and
    /// returns the one-shot scripted `fail` error (consumed on the first
    /// call). P6.F3: `scripted` holds per-call results, consumed from the
    /// back (one `pop` per `run` call), before falling back to `fail` and
    /// then `Ok(())`; it lets a test script "first call fails with
    /// `Waited`, second call succeeds" without any real process.
    struct MockRunner {
        ran: Mutex<Vec<String>>,
        fail: Mutex<Option<RunError>>,
        scripted: Mutex<Vec<Result<(), RunError>>>,
    }

    impl CommandRunner for MockRunner {
        fn run(&self, entry: &QueueEntry) -> Result<(), RunError> {
            self.ran.lock().unwrap().push(entry.as_str().to_owned());
            match self.scripted.lock().unwrap().pop() {
                Some(result) => result,
                None => self.fail.lock().unwrap().take().map_or(Ok(()), Err),
            }
        }
    }

    /// `EditorLauncher` mock: records each `QueuePath` `open` was called with
    /// and returns the one-shot scripted `fail` error (consumed on the first
    /// call). No real editor is spawned.
    struct MockEditor {
        opened: Mutex<Vec<PathBuf>>,
        fail: Mutex<Option<EditorError>>,
    }

    impl EditorLauncher for MockEditor {
        fn open(&self, at: &QueuePath) -> Result<(), EditorError> {
            self.opened.lock().unwrap().push(at.as_path().to_path_buf());
            self.fail.lock().unwrap().take().map_or(Ok(()), Err)
        }
    }

    /// `QueueLock` mock (P6.F4): records `acquire`/`release` call counts and
    /// returns the one-shot scripted `acquire_err`/`release_err` (consumed on
    /// the next matching call). Purely in-memory.
    struct MockLock {
        acquire_calls: Mutex<u64>,
        release_calls: Mutex<u64>,
        acquire_err: Mutex<Option<LockError>>,
        release_err: Mutex<Option<LockError>>,
    }

    impl QueueLock for MockLock {
        fn acquire(&self) -> Result<(), LockError> {
            *self.acquire_calls.lock().unwrap() += 1;
            self.acquire_err.lock().unwrap().take().map_or(Ok(()), Err)
        }

        fn release(&self) -> Result<(), LockError> {
            *self.release_calls.lock().unwrap() += 1;
            self.release_err.lock().unwrap().take().map_or(Ok(()), Err)
        }
    }

    fn mock_lock() -> MockLock {
        MockLock {
            acquire_calls: Mutex::new(0),
            release_calls: Mutex::new(0),
            acquire_err: Mutex::new(None),
            release_err: Mutex::new(None),
        }
    }

    fn entry(line: &str) -> QueueEntry {
        QueueEntry::new(line).unwrap()
    }

    /// P6.F3: seconds since the Unix epoch for the test process (test-only
    /// `expect`; TST-03), used to bound the failure-record timestamp window.
    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after the Unix epoch")
            .as_secs()
    }

    #[test]
    fn add_appends_entry_to_store() {
        let store = MockStore {
            entries: Mutex::new(Vec::new()),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let service = QueueService::new(
            store,
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
                scripted: Mutex::new(Vec::new()),
            },
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            mock_lock(),
        );

        service.add(&entry("nix build .")).unwrap();

        let entries = service.ls().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].as_str(), "nix build .");
    }

    #[test]
    fn ls_returns_entries_in_order_including_blank_line() {
        let store = MockStore {
            entries: Mutex::new(vec![entry("a"), entry("   "), entry("c")]),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let service = QueueService::new(
            store,
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
                scripted: Mutex::new(Vec::new()),
            },
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            mock_lock(),
        );

        let lines: Vec<String> = service
            .ls()
            .unwrap()
            .iter()
            .map(|e| e.as_str().to_owned())
            .collect();
        assert_eq!(
            lines,
            vec!["a".to_owned(), "   ".to_owned(), "c".to_owned()]
        );
    }

    #[test]
    fn start_removes_entry_after_successful_run() {
        let store = MockStore {
            entries: Mutex::new(vec![entry("cmd")]),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let runner = MockRunner {
            ran: Mutex::new(Vec::new()),
            fail: Mutex::new(None),
            scripted: Mutex::new(Vec::new()),
        };
        let service = QueueService::new(
            store,
            runner,
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            mock_lock(),
        );

        service.start(false).unwrap();

        assert_eq!(service.ls().unwrap(), Vec::<QueueEntry>::new());
        assert_eq!(*service.runner.ran.lock().unwrap(), vec!["cmd".to_owned()]);
    }

    #[test]
    fn start_runs_each_entry_until_queue_empty() {
        let store = MockStore {
            entries: Mutex::new(vec![entry("first"), entry("second")]),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let runner = MockRunner {
            ran: Mutex::new(Vec::new()),
            fail: Mutex::new(None),
            scripted: Mutex::new(Vec::new()),
        };
        let service = QueueService::new(
            store,
            runner,
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            mock_lock(),
        );

        service.start(false).unwrap();

        assert_eq!(
            *service.runner.ran.lock().unwrap(),
            vec!["first".to_owned(), "second".to_owned()]
        );
        assert!(service.ls().unwrap().is_empty());
    }

    #[test]
    fn start_trims_entry_before_running() {
        let store = MockStore {
            entries: Mutex::new(vec![entry("  cmd  ")]),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let runner = MockRunner {
            ran: Mutex::new(Vec::new()),
            fail: Mutex::new(None),
            scripted: Mutex::new(Vec::new()),
        };
        let service = QueueService::new(
            store,
            runner,
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            mock_lock(),
        );

        service.start(false).unwrap();

        assert_eq!(*service.runner.ran.lock().unwrap(), vec!["cmd".to_owned()]);
        assert!(service.ls().unwrap().is_empty());
    }

    #[test]
    fn start_skips_blank_first_line_but_still_removes_it() {
        let store = MockStore {
            entries: Mutex::new(vec![entry("   "), entry("cmd")]),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let runner = MockRunner {
            ran: Mutex::new(Vec::new()),
            fail: Mutex::new(None),
            scripted: Mutex::new(Vec::new()),
        };
        let service = QueueService::new(
            store,
            runner,
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            mock_lock(),
        );

        service.start(false).unwrap();

        // Exactly one run: the blank first line is removed without being run,
        // then "cmd" runs.
        assert_eq!(*service.runner.ran.lock().unwrap(), vec!["cmd".to_owned()]);
        assert!(service.ls().unwrap().is_empty());
    }

    /// P6.F3: a non-zero exit (`Waited { code: 3 }`) WITHOUT `stop_on_error`
    /// logs the failure (record fields asserted), drops the failed entry,
    /// and the loop continues and runs the next entry, ending `Ok(())`.
    #[test]
    fn start_waited_without_flag_logs_drops_and_continues() {
        let before = now_secs();
        let store = MockStore {
            entries: Mutex::new(vec![entry("cmd"), entry("next")]),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let runner = MockRunner {
            ran: Mutex::new(Vec::new()),
            fail: Mutex::new(None),
            // `pop()` consumes from the back: the first `run` call fails
            // with `Waited { code: 3 }`; the second call (script exhausted)
            // falls back to `Ok(())`.
            scripted: Mutex::new(vec![Err(RunError::Waited { code: 3 })]),
        };
        let service = QueueService::new(
            store,
            runner,
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            mock_lock(),
        );

        assert!(matches!(service.start(false), Ok(())));

        // Exactly one failure record, with the runner's exit code and the
        // trimmed entry that was executed.
        let failures = service.store.failures.lock().unwrap();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].exit_code, 3);
        assert_eq!(failures[0].command.as_str(), "cmd");
        // Timestamp within a sane window around the test (no exact
        // wall-clock assertion).
        let after = now_secs();
        assert!(
            failures[0].timestamp >= before && failures[0].timestamp <= after,
            "recorded {} outside [{before}, {after}]",
            failures[0].timestamp
        );
        drop(failures);

        // The failed entry was dropped by the failure path, and the next
        // entry by the loop's unconditional removal after it ran: two
        // removals in total. The loop continued: the next entry ran, and
        // the queue is now empty.
        assert_eq!(*service.store.drop_calls.lock().unwrap(), 2);
        assert_eq!(
            *service.runner.ran.lock().unwrap(),
            vec!["cmd".to_owned(), "next".to_owned()]
        );
        assert!(service.ls().unwrap().is_empty());
    }

    /// P6.F3: a non-zero exit WITH `stop_on_error` logs the failure, drops
    /// the entry, and returns `Err(QueueError::StoppedOnFailure)` — and the
    /// runner is NOT called a second time.
    #[test]
    fn start_waited_with_flag_stops_after_logging() {
        let store = MockStore {
            entries: Mutex::new(vec![entry("cmd"), entry("second")]),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let runner = MockRunner {
            ran: Mutex::new(Vec::new()),
            fail: Mutex::new(None),
            // `pop()` consumes from the back: if the runner were
            // (incorrectly) called a second time, it would take this
            // `Ok(())` and the loop would run past the failure — so a
            // second `ran` entry proves it. The first call takes the
            // `Waited`.
            scripted: Mutex::new(vec![Ok(()), Err(RunError::Waited { code: 1 })]),
        };
        let service = QueueService::new(
            store,
            runner,
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            mock_lock(),
        );

        let err = service.start(true).unwrap_err();
        assert!(matches!(err, QueueError::StoppedOnFailure));

        let failures = service.store.failures.lock().unwrap();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].exit_code, 1);
        assert_eq!(failures[0].command.as_str(), "cmd");
        drop(failures);
        assert_eq!(*service.store.drop_calls.lock().unwrap(), 1);

        // The runner was called exactly ONCE: it was not called a second
        // time.
        assert_eq!(*service.runner.ran.lock().unwrap(), vec!["cmd".to_owned()]);
        // The failed entry was removed; the next one is still queued.
        assert_eq!(
            entry_lines(&service.ls().unwrap()),
            vec!["second".to_owned()]
        );
    }

    /// P6.F3: a `Spawn` error is unchanged behaviour — the command never
    /// ran, so NO `append_failure` and NO `drop_first` call happens, the
    /// entry stays queued, and the error propagates.
    #[test]
    fn start_spawn_error_logs_nothing_and_keeps_entry() {
        let store = MockStore {
            entries: Mutex::new(vec![entry("cmd"), entry("second")]),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let runner = MockRunner {
            ran: Mutex::new(Vec::new()),
            fail: Mutex::new(Some(RunError::Spawn(std::io::Error::other("no shell")))),
            // `scripted` is empty so the one-shot `fail` is returned on the
            // first call.
            scripted: Mutex::new(Vec::new()),
        };
        let service = QueueService::new(
            store,
            runner,
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            mock_lock(),
        );

        let err = service.start(false).unwrap_err();
        assert!(matches!(err, QueueError::Run(RunError::Spawn(_))));

        // No failure was logged and nothing was dropped; the queue is
        // untouched.
        assert!(service.store.failures.lock().unwrap().is_empty());
        assert_eq!(*service.store.drop_calls.lock().unwrap(), 0);
        assert_eq!(
            entry_lines(&service.ls().unwrap()),
            vec!["cmd".to_owned(), "second".to_owned()]
        );
    }

    /// P6.F3 regression guard: an all-successful `start` makes zero
    /// `append_failure` calls.
    #[test]
    fn start_success_path_logs_no_failures() {
        let store = MockStore {
            entries: Mutex::new(vec![entry("a"), entry("b")]),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let runner = MockRunner {
            ran: Mutex::new(Vec::new()),
            fail: Mutex::new(None),
            scripted: Mutex::new(Vec::new()),
        };
        let service = QueueService::new(
            store,
            runner,
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            mock_lock(),
        );

        assert!(matches!(service.start(false), Ok(())));

        assert!(service.store.failures.lock().unwrap().is_empty());
        assert_eq!(
            *service.runner.ran.lock().unwrap(),
            vec!["a".to_owned(), "b".to_owned()]
        );
        assert!(service.ls().unwrap().is_empty());
    }

    #[test]
    fn start_propagates_runner_error_and_keeps_entry() {
        let store = MockStore {
            entries: Mutex::new(vec![entry("cmd"), entry("second")]),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let runner = MockRunner {
            ran: Mutex::new(Vec::new()),
            fail: Mutex::new(Some(RunError::Other(std::io::Error::other("spawn failed")))),
            scripted: Mutex::new(Vec::new()),
        };
        let service = QueueService::new(
            store,
            runner,
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            mock_lock(),
        );

        let err = service.start(false).unwrap_err();
        assert!(matches!(err, QueueError::Run(RunError::Other(_))));

        // Error is before the rewrite: the store STILL contains the entry.
        let lines: Vec<String> = service
            .ls()
            .unwrap()
            .iter()
            .map(|e| e.as_str().to_owned())
            .collect();
        assert_eq!(lines, vec!["cmd".to_owned(), "second".to_owned()]);
    }

    #[test]
    fn start_propagates_store_error_without_running() {
        let store = MockStore {
            entries: Mutex::new(vec![entry("cmd")]),
            read_err: Mutex::new(Some(QueueStoreError::QueueFileMissing(PathBuf::from(
                "/tmp/mock-home/queue/commands.txt",
            )))),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let runner = MockRunner {
            ran: Mutex::new(Vec::new()),
            fail: Mutex::new(None),
            scripted: Mutex::new(Vec::new()),
        };
        let service = QueueService::new(
            store,
            runner,
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            mock_lock(),
        );

        let err = service.start(false).unwrap_err();
        assert!(matches!(
            err,
            QueueError::Store(QueueStoreError::QueueFileMissing(_))
        ));

        // The read failed before any run: no run call was made.
        assert!(service.runner.ran.lock().unwrap().is_empty());
    }

    /// P6.F4: a held lock (`HeldBy(7)`) refuses `start` before any store
    /// access — the scripted one-shot `read_err` would surface as a
    /// `Store` error if `read_all` were ever called, and the drain error
    /// wins over the release in `start`, so the returned `Lock` error
    /// proves `read_all` never ran. The release is not attempted.
    #[test]
    fn start_acquire_held_by_returns_lock_error_without_touching_the_store() {
        let store = MockStore {
            entries: Mutex::new(vec![entry("cmd")]),
            read_err: Mutex::new(Some(QueueStoreError::Io(std::io::Error::other(
                "read must not be attempted",
            )))),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let lock = MockLock {
            acquire_calls: Mutex::new(0),
            release_calls: Mutex::new(0),
            acquire_err: Mutex::new(Some(LockError::HeldBy(7))),
            release_err: Mutex::new(None),
        };
        let service = QueueService::new(
            store,
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
                scripted: Mutex::new(Vec::new()),
            },
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            lock,
        );

        let err = service.start(false).unwrap_err();
        assert!(matches!(err, QueueError::Lock(LockError::HeldBy(7))));

        assert_eq!(*service.lock.acquire_calls.lock().unwrap(), 1);
        // Release is not attempted after a failed acquire.
        assert_eq!(*service.lock.release_calls.lock().unwrap(), 0);
    }

    /// P6.F4: a drain error returns the drain error and the release is
    /// still called exactly once.
    #[test]
    fn start_drain_error_still_releases_the_lock_once() {
        let store = MockStore {
            entries: Mutex::new(vec![entry("cmd")]),
            read_err: Mutex::new(Some(QueueStoreError::QueueFileMissing(PathBuf::from(
                "/tmp/mock-home/queue/commands.txt",
            )))),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let lock = mock_lock();
        let service = QueueService::new(
            store,
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
                scripted: Mutex::new(Vec::new()),
            },
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            lock,
        );

        let err = service.start(false).unwrap_err();
        assert!(matches!(
            err,
            QueueError::Store(QueueStoreError::QueueFileMissing(_))
        ));

        assert_eq!(*service.lock.acquire_calls.lock().unwrap(), 1);
        assert_eq!(*service.lock.release_calls.lock().unwrap(), 1);
    }

    /// P6.F4: a successful drain releases the lock exactly once.
    #[test]
    fn start_success_releases_the_lock_once() {
        let service = make_service(vec![entry("a"), entry("b")]);

        assert!(matches!(service.start(false), Ok(())));

        assert_eq!(*service.lock.acquire_calls.lock().unwrap(), 1);
        assert_eq!(*service.lock.release_calls.lock().unwrap(), 1);
        assert!(service.ls().unwrap().is_empty());
    }

    /// P6.F4: a release failure after a successful drain surfaces as
    /// `QueueError::Lock(LockError::Io(_))`; the drain itself completed.
    #[test]
    fn start_release_failure_after_success_is_lock_error() {
        let store = MockStore {
            entries: Mutex::new(vec![entry("a")]),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let lock = MockLock {
            acquire_calls: Mutex::new(0),
            release_calls: Mutex::new(0),
            acquire_err: Mutex::new(None),
            release_err: Mutex::new(Some(LockError::Io(std::io::Error::other(
                "could not remove the lock",
            )))),
        };
        let service = QueueService::new(
            store,
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
                scripted: Mutex::new(Vec::new()),
            },
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            lock,
        );

        let err = service.start(false).unwrap_err();
        assert!(matches!(err, QueueError::Lock(LockError::Io(_))));

        // The drain completed before the release failed: the queue is
        // drained.
        assert!(service.ls().unwrap().is_empty());
        assert_eq!(*service.lock.release_calls.lock().unwrap(), 1);
    }

    #[test]
    fn edit_opens_editor_at_given_path() {
        let store = MockStore {
            entries: Mutex::new(Vec::new()),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let editor = MockEditor {
            opened: Mutex::new(Vec::new()),
            fail: Mutex::new(None),
        };
        let service = QueueService::new(
            store,
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
                scripted: Mutex::new(Vec::new()),
            },
            editor,
            mock_lock(),
        );

        let at = QueuePath::new(PathBuf::from("/tmp/mock-home/queue/commands.txt"));
        service.edit(&at).unwrap();

        let opened = service.editor.opened.lock().unwrap();
        assert_eq!(opened.len(), 1);
        assert_eq!(
            opened[0],
            PathBuf::from("/tmp/mock-home/queue/commands.txt")
        );
    }

    #[test]
    fn edit_propagates_editor_error() {
        let store = MockStore {
            entries: Mutex::new(Vec::new()),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let editor = MockEditor {
            opened: Mutex::new(Vec::new()),
            fail: Mutex::new(Some(EditorError::Spawn(std::io::Error::other("no editor")))),
        };
        let service = QueueService::new(
            store,
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
                scripted: Mutex::new(Vec::new()),
            },
            editor,
            mock_lock(),
        );

        let at = QueuePath::new(PathBuf::from("/tmp/mock-home/queue/commands.txt"));
        let err = service.edit(&at).unwrap_err();
        assert!(matches!(err, QueueError::Editor(EditorError::Spawn(_))));
    }

    /// P6 (V-ARCH-05 behaviour half): `ls` on a **missing** queue file no
    /// longer errors — it reports an empty queue. (The `start` missing-file
    /// expectation is unchanged: see `start_propagates_store_error_without_running`.)
    #[test]
    fn ls_on_missing_file_returns_empty() {
        let store = MockStore {
            entries: Mutex::new(Vec::new()),
            read_err: Mutex::new(Some(QueueStoreError::QueueFileMissing(PathBuf::from(
                "/tmp/mock-home/queue/commands.txt",
            )))),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let service = QueueService::new(
            store,
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
                scripted: Mutex::new(Vec::new()),
            },
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            mock_lock(),
        );

        assert_eq!(service.ls().unwrap(), Vec::<QueueEntry>::new());
    }

    /// Only the missing-file case is softened for `ls`; any other store
    /// error still propagates as `QueueError::Store` (P6).
    #[test]
    fn ls_propagates_other_store_errors() {
        let store = MockStore {
            entries: Mutex::new(Vec::new()),
            read_err: Mutex::new(Some(QueueStoreError::Io(std::io::Error::other(
                "permission denied",
            )))),
            drop_err: Mutex::new(None),
            failures: Mutex::new(Vec::new()),
            drop_calls: Mutex::new(0),
        };
        let service = QueueService::new(
            store,
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
                scripted: Mutex::new(Vec::new()),
            },
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            mock_lock(),
        );

        let err = service.ls().unwrap_err();
        assert!(matches!(err, QueueError::Store(QueueStoreError::Io(_))));
    }

    fn make_service(
        entries: Vec<QueueEntry>,
    ) -> QueueService<MockStore, MockRunner, MockEditor, MockLock> {
        QueueService::new(
            MockStore {
                entries: Mutex::new(entries),
                read_err: Mutex::new(None),
                drop_err: Mutex::new(None),
                failures: Mutex::new(Vec::new()),
                drop_calls: Mutex::new(0),
            },
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
                scripted: Mutex::new(Vec::new()),
            },
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            mock_lock(),
        )
    }

    fn entry_lines(entries: &[QueueEntry]) -> Vec<String> {
        entries.iter().map(|e| e.as_str().to_owned()).collect()
    }

    /// Build a `QueueEntryIndex` from a decimal literal (test-only; TST-03).
    fn index(s: &str) -> QueueEntryIndex {
        QueueEntryIndex::from_str(s).expect("valid index literal")
    }

    #[test]
    fn remove_entry_removes_middle_and_keeps_order() {
        let service = make_service(vec![entry("a"), entry("b"), entry("c")]);

        service.remove_entry(index("2")).unwrap();

        assert_eq!(
            entry_lines(&service.ls().unwrap()),
            vec!["a".to_owned(), "c".to_owned()]
        );
    }

    #[test]
    fn remove_entry_removes_first_and_last() {
        // First.
        let service = make_service(vec![entry("a"), entry("b"), entry("c")]);
        service.remove_entry(index("1")).unwrap();
        assert_eq!(
            entry_lines(&service.ls().unwrap()),
            vec!["b".to_owned(), "c".to_owned()]
        );

        // Last.
        let service = make_service(vec![entry("a"), entry("b"), entry("c")]);
        service.remove_entry(index("3")).unwrap();
        assert_eq!(
            entry_lines(&service.ls().unwrap()),
            vec!["a".to_owned(), "b".to_owned()]
        );
    }

    #[test]
    fn remove_entry_removing_the_only_entry_leaves_an_empty_queue() {
        let service = make_service(vec![entry("a")]);

        service.remove_entry(index("1")).unwrap();

        assert!(service.ls().unwrap().is_empty());
    }

    #[test]
    fn remove_entry_out_of_range_is_no_entry_and_leaves_the_store_untouched() {
        // `pos > len`. (The `pos == 0` case is now unconstructible at the
        // type level: `QueueEntryIndex` rejects 0 at parse time, so only the
        // `> len` half of the guard is reachable through the public API.)
        let service = make_service(vec![entry("a"), entry("b"), entry("c")]);
        let err = service.remove_entry(index("4")).unwrap_err();
        assert!(matches!(err, QueueError::NoEntry(QueueEntryIndex(i)) if i == 4));
        assert_eq!(
            entry_lines(&service.ls().unwrap()),
            vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
        );
    }

    #[test]
    fn remove_entry_on_empty_queue_is_no_entry() {
        let service = make_service(Vec::new());
        let err = service.remove_entry(index("1")).unwrap_err();
        assert!(matches!(err, QueueError::NoEntry(QueueEntryIndex(i)) if i == 1));
    }

    #[test]
    fn remove_entry_on_missing_file_propagates_the_store_error() {
        let service = QueueService::new(
            MockStore {
                entries: Mutex::new(Vec::new()),
                read_err: Mutex::new(Some(QueueStoreError::QueueFileMissing(PathBuf::from(
                    "/tmp/mock-home/queue/commands.txt",
                )))),
                drop_err: Mutex::new(None),
                failures: Mutex::new(Vec::new()),
                drop_calls: Mutex::new(0),
            },
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
                scripted: Mutex::new(Vec::new()),
            },
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            mock_lock(),
        );

        let err = service.remove_entry(index("1")).unwrap_err();
        assert!(matches!(
            err,
            QueueError::Store(QueueStoreError::QueueFileMissing(_))
        ));
    }

    #[test]
    fn queue_error_composes_each_boundary_error_via_from() {
        // Store boundary.
        let store: QueueError = QueueStoreError::QueueFileMissing(PathBuf::from("q")).into();
        assert!(matches!(
            store,
            QueueError::Store(QueueStoreError::QueueFileMissing(_))
        ));

        // Runner boundary.
        let run: QueueError = RunError::Other(std::io::Error::other("boom")).into();
        assert!(matches!(run, QueueError::Run(RunError::Other(_))));

        // Editor boundary.
        let editor: QueueError = EditorError::Spawn(std::io::Error::other("no editor")).into();
        assert!(matches!(editor, QueueError::Editor(EditorError::Spawn(_))));

        // Domain boundary (the deferred strict `Empty` reject).
        let invalid: QueueError = QueueEntryError::Empty.into();
        assert!(matches!(
            invalid,
            QueueError::Invalid(QueueEntryError::Empty)
        ));

        // Lock boundary (P6.F4).
        let lock: QueueError = LockError::HeldBy(7).into();
        assert!(matches!(lock, QueueError::Lock(LockError::HeldBy(7))));
    }
}
