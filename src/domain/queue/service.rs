//! Application/service layer for the queue.
//!
//! WP08: `QueueService` is generic over its three injected ports
//! (`R: QueueStore`, `C: CommandRunner`, `E: EditorLauncher`) and exposes
//! `add`/`ls`/`start`/`edit`. Every method takes and returns only domain
//! newtypes and the [`QueueError`] enum (ARCH-04): no `&str`/`String`
//! parameters, and the module imports only `crate::domain` and `crate::ports`
//! (ARCH-01).
//! `QueueError` composes the four boundary enums via `#[from]` and never
//! duplicates their variants (ERR-02).
//!
//! A third port generic (`E: EditorLauncher`) beyond the `QueueService<R, C>`
//! shorthand is intentional: it lets all three ports be injected with mocks
//! (TST-01), which the two-generic form cannot express.
//!
//! `start` preserves the legacy `process_queue` behaviour byte-for-byte:
//! trim-before-run, unconditional removal of the first line (blank lines are
//! removed, not run), a fresh re-read + rewrite each iteration, immediate
//! error propagation before the rewrite, and a discarded child exit status.
//! The strict trim + `Empty`-reject (V-TY-02) and exit-status surfacing
//! (V-ERR-09) are deferred P6 changes and are NOT implemented here.

use crate::domain::queue::{QueueEntry, QueueEntryError, QueuePath};
use crate::ports::command_runner::{CommandRunner, RunError};
use crate::ports::editor::{EditorError, EditorLauncher};
use crate::ports::queue_store::{QueueStore, QueueStoreError};

/// The queue service, generic over its three injected ports.
///
/// A third port generic (`E: EditorLauncher`) beyond the `QueueService<R, C>`
/// shorthand is required so that all three ports can be injected with mocks
/// (TST-01). The queue `QueuePath` is deliberately NOT a field here: `Config`
/// remains the single origin of that path (V-ARCH-05), so the service depends
/// only on the ports (ARCH-01).
#[derive(Debug, Clone)]
pub struct QueueService<R, C, E>
where
    R: QueueStore,
    C: CommandRunner,
    E: EditorLauncher,
{
    store: R,
    runner: C,
    editor: E,
}

/// Errors surfaced by the queue service.
///
/// Composes the four boundary enums via `#[from]` and never duplicates their
/// variants (ERR-02): `Store` for queue-file I/O, `Run` for command execution,
/// `Editor` for editor launch, and `Invalid` for a line rejected by the
/// domain's `QueueEntry` constructor.
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
    NoEntry(u64),
}

impl<R, C, E> QueueService<R, C, E>
where
    R: QueueStore,
    C: CommandRunner,
    E: EditorLauncher,
{
    /// Wire the three ports into a service.
    pub fn new(store: R, runner: C, editor: E) -> Self {
        Self {
            store,
            runner,
            editor,
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

    /// Run the queued entries, one per iteration, until the queue is empty.
    ///
    /// Each iteration reads the whole queue (a missing file surfaces as
    /// `QueueError::Store`), breaks on an empty queue without rewriting, trims
    /// the first line and runs it only when non-blank (a blank first line is
    /// not run but is still removed — the removal is unconditional), then drops
    /// the first entry by re-reading and rewriting the remainder. A store or
    /// runner error propagates immediately, before the rewrite, leaving the
    /// queue unmodified up to that point. The child's exit status is discarded
    /// (WP05-preserved; V-ERR-09 is deferred to P6).
    pub fn start(&self) -> Result<(), QueueError> {
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
                // The child's exit status is discarded here (WP05-preserved).
                self.runner.run(&trimmed)?;
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
    pub fn remove_entry(&self, pos: u64) -> Result<(), QueueError> {
        let entries = self.store.read_all()?;
        if pos == 0 || pos > entries.len() as u64 {
            return Err(QueueError::NoEntry(pos));
        }
        let target = pos as usize - 1;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Mutex;

    /// In-memory `QueueStore` mock: `entries` is the backing queue; `read_err`
    /// and `drop_err` are one-shot scripted failures (returned on the next
    /// matching call). Purely in-memory: no real file, no environment access.
    struct MockStore {
        entries: Mutex<Vec<QueueEntry>>,
        read_err: Mutex<Option<QueueStoreError>>,
        drop_err: Mutex<Option<QueueStoreError>>,
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
    }

    /// `CommandRunner` mock: records each `as_str()` it was asked to run and
    /// returns the one-shot scripted `fail` error (consumed on the first call).
    struct MockRunner {
        ran: Mutex<Vec<String>>,
        fail: Mutex<Option<RunError>>,
    }

    impl CommandRunner for MockRunner {
        fn run(&self, entry: &QueueEntry) -> Result<(), RunError> {
            self.ran.lock().unwrap().push(entry.as_str().to_owned());
            self.fail.lock().unwrap().take().map_or(Ok(()), Err)
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

    fn entry(line: &str) -> QueueEntry {
        QueueEntry::new(line).unwrap()
    }

    #[test]
    fn add_appends_entry_to_store() {
        let store = MockStore {
            entries: Mutex::new(Vec::new()),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
        };
        let service = QueueService::new(
            store,
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
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
        };
        let service = QueueService::new(
            store,
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
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
        };
        let runner = MockRunner {
            ran: Mutex::new(Vec::new()),
            fail: Mutex::new(None),
        };
        let service = QueueService::new(
            store,
            runner,
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
        );

        service.start().unwrap();

        assert_eq!(service.ls().unwrap(), Vec::<QueueEntry>::new());
        assert_eq!(*service.runner.ran.lock().unwrap(), vec!["cmd".to_owned()]);
    }

    #[test]
    fn start_runs_each_entry_until_queue_empty() {
        let store = MockStore {
            entries: Mutex::new(vec![entry("first"), entry("second")]),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
        };
        let runner = MockRunner {
            ran: Mutex::new(Vec::new()),
            fail: Mutex::new(None),
        };
        let service = QueueService::new(
            store,
            runner,
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
        );

        service.start().unwrap();

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
        };
        let runner = MockRunner {
            ran: Mutex::new(Vec::new()),
            fail: Mutex::new(None),
        };
        let service = QueueService::new(
            store,
            runner,
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
        );

        service.start().unwrap();

        assert_eq!(*service.runner.ran.lock().unwrap(), vec!["cmd".to_owned()]);
        assert!(service.ls().unwrap().is_empty());
    }

    #[test]
    fn start_skips_blank_first_line_but_still_removes_it() {
        let store = MockStore {
            entries: Mutex::new(vec![entry("   "), entry("cmd")]),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
        };
        let runner = MockRunner {
            ran: Mutex::new(Vec::new()),
            fail: Mutex::new(None),
        };
        let service = QueueService::new(
            store,
            runner,
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
        );

        service.start().unwrap();

        // Exactly one run: the blank first line is removed without being run,
        // then "cmd" runs.
        assert_eq!(*service.runner.ran.lock().unwrap(), vec!["cmd".to_owned()]);
        assert!(service.ls().unwrap().is_empty());
    }

    #[test]
    fn start_propagates_runner_error_and_keeps_entry() {
        let store = MockStore {
            entries: Mutex::new(vec![entry("cmd"), entry("second")]),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
        };
        let runner = MockRunner {
            ran: Mutex::new(Vec::new()),
            fail: Mutex::new(Some(RunError::Other(std::io::Error::other("spawn failed")))),
        };
        let service = QueueService::new(
            store,
            runner,
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
        );

        let err = service.start().unwrap_err();
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
        };
        let runner = MockRunner {
            ran: Mutex::new(Vec::new()),
            fail: Mutex::new(None),
        };
        let service = QueueService::new(
            store,
            runner,
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
        );

        let err = service.start().unwrap_err();
        assert!(matches!(
            err,
            QueueError::Store(QueueStoreError::QueueFileMissing(_))
        ));

        // The read failed before any run: no run call was made.
        assert!(service.runner.ran.lock().unwrap().is_empty());
    }

    #[test]
    fn edit_opens_editor_at_given_path() {
        let store = MockStore {
            entries: Mutex::new(Vec::new()),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
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
            },
            editor,
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
            },
            editor,
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
        };
        let service = QueueService::new(
            store,
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
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
        };
        let service = QueueService::new(
            store,
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
        );

        let err = service.ls().unwrap_err();
        assert!(matches!(err, QueueError::Store(QueueStoreError::Io(_))));
    }

    fn make_service(entries: Vec<QueueEntry>) -> QueueService<MockStore, MockRunner, MockEditor> {
        QueueService::new(
            MockStore {
                entries: Mutex::new(entries),
                read_err: Mutex::new(None),
                drop_err: Mutex::new(None),
            },
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
        )
    }

    fn entry_lines(entries: &[QueueEntry]) -> Vec<String> {
        entries.iter().map(|e| e.as_str().to_owned()).collect()
    }

    #[test]
    fn remove_entry_removes_middle_and_keeps_order() {
        let service = make_service(vec![entry("a"), entry("b"), entry("c")]);

        service.remove_entry(2).unwrap();

        assert_eq!(
            entry_lines(&service.ls().unwrap()),
            vec!["a".to_owned(), "c".to_owned()]
        );
    }

    #[test]
    fn remove_entry_removes_first_and_last() {
        // First.
        let service = make_service(vec![entry("a"), entry("b"), entry("c")]);
        service.remove_entry(1).unwrap();
        assert_eq!(
            entry_lines(&service.ls().unwrap()),
            vec!["b".to_owned(), "c".to_owned()]
        );

        // Last.
        let service = make_service(vec![entry("a"), entry("b"), entry("c")]);
        service.remove_entry(3).unwrap();
        assert_eq!(
            entry_lines(&service.ls().unwrap()),
            vec!["a".to_owned(), "b".to_owned()]
        );
    }

    #[test]
    fn remove_entry_removing_the_only_entry_leaves_an_empty_queue() {
        let service = make_service(vec![entry("a")]);

        service.remove_entry(1).unwrap();

        assert!(service.ls().unwrap().is_empty());
    }

    #[test]
    fn remove_entry_out_of_range_is_no_entry_and_leaves_the_store_untouched() {
        // `pos == 0`.
        let service = make_service(vec![entry("a"), entry("b"), entry("c")]);
        let err = service.remove_entry(0).unwrap_err();
        assert!(matches!(err, QueueError::NoEntry(0)));
        assert_eq!(
            entry_lines(&service.ls().unwrap()),
            vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
        );

        // `pos > len`.
        let service = make_service(vec![entry("a"), entry("b"), entry("c")]);
        let err = service.remove_entry(4).unwrap_err();
        assert!(matches!(err, QueueError::NoEntry(4)));
        assert_eq!(
            entry_lines(&service.ls().unwrap()),
            vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
        );
    }

    #[test]
    fn remove_entry_on_empty_queue_is_no_entry() {
        let service = make_service(Vec::new());
        let err = service.remove_entry(1).unwrap_err();
        assert!(matches!(err, QueueError::NoEntry(1)));
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
            },
            MockRunner {
                ran: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
            MockEditor {
                opened: Mutex::new(Vec::new()),
                fail: Mutex::new(None),
            },
        );

        let err = service.remove_entry(1).unwrap_err();
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
    }
}
