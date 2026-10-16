//! Inbound CLI adapter (WP09; ARCH-04, ARCH-03, ERR-02/ERR-05).
//!
//! Role: the `clap` boundary of `encode_queue`. It parses the
//! `add`/`start`/`ls`/`edit` subcommands, translates the raw `COMMAND`
//! argument into a `QueueEntry` **at the boundary**
//! (`QueueEntry::from_str(cmd).map_err(CliError::BadCommand)`), and dispatches
//! to the `QueueService`. Business code never sees the raw CLI string. The
//! `queue_path` is injected (never computed here — `Config` is the single
//! origin, V-ARCH-05) and is used only by the `edit` dispatch.
//!
//! Parse behaviour: `run()` calls `command().get_matches()`, which exits the
//! process with code 2 on a usage error itself — exactly like the legacy
//! binary's `Command::get_matches()` in `main`. The builder configuration
//! (name, version, author, about strings, `subcommand_required(true)`, the
//! four subcommands and the required positional `COMMAND` argument) is copied
//! verbatim from the legacy binary so help/usage/error output stays
//! byte-identical.
//!
//! Error mapping: every failure becomes one of the three `CliError` variants
//! — `BadCommand` (a raw argument failed `QueueEntry::from_str`; the lenient
//! P5 constructor never fails, so this is wired for the deferred
//! strict-constructor path), `MissingArgument` (a required argument was
//! absent from the parsed matches — the V-ERR-02 `unwrap`-free replacement),
//! and `Service` (`#[from] QueueError`). Display strings are not the API
//! (ERR-05); WP10 maps the variants to messages and exit codes. There is no
//! I/O variant: the `ls` dispatch writes through the `print!` macro, which
//! swallows write errors, precisely like the legacy `println!` macro.
//!
//! `ls` output rule: every entry verbatim, one per line, followed by a final
//! blank line — byte-identical to the legacy `println!("{}", contents)` for
//! any queue file this tool writes (always trailing-newline-terminated; see
//! `ls_output`).

use clap::{Arg, ArgAction, ArgMatches, Command};

use crate::domain::queue::service::{QueueError, QueueService};
use crate::domain::queue::{QueueEntry, QueueEntryError, QueueEntryIndex, QueuePath};
use crate::ports::command_runner::CommandRunner;
use crate::ports::editor::EditorLauncher;
use crate::ports::queue_store::QueueStore;
use std::str::FromStr;

/// The inbound CLI adapter: clap parsing at the boundary plus dispatch to
/// the queue service.
pub struct Cli<R: QueueStore, C: CommandRunner, E: EditorLauncher> {
    service: QueueService<R, C, E>,
    /// The queue-file path, injected by the caller (`Config` is the single
    /// origin, V-ARCH-05) and used only by the `edit` dispatch.
    queue_path: QueuePath,
}

/// Errors surfaced by the inbound CLI adapter.
///
/// Exactly three variants (ERR-02: one enum per boundary). Display strings
/// are not the API (ERR-05): WP10 maps these variants to messages and exit
/// codes.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// A raw CLI argument failed to parse into a `QueueEntry`.
    #[error(transparent)]
    BadCommand(#[from] QueueEntryError),
    /// A required argument was not present in the parsed matches.
    #[error("missing required argument: {0}")]
    MissingArgument(String),
    /// The service layer reported an error.
    #[error(transparent)]
    Service(#[from] QueueError),
}

impl<R, C, E> Cli<R, C, E>
where
    R: QueueStore,
    C: CommandRunner,
    E: EditorLauncher,
{
    /// Wire the service and the queue path into the CLI adapter.
    pub fn new(service: QueueService<R, C, E>, queue_path: QueuePath) -> Self {
        Self {
            service,
            queue_path,
        }
    }

    /// Parse the arguments and dispatch.
    ///
    /// `get_matches` exits the process itself (code 2) on a parse error —
    /// the same behaviour as the legacy binary.
    pub fn run(&self) -> Result<(), CliError> {
        let matches = command().get_matches();
        self.dispatch(&matches)
    }

    /// Dispatch the parsed subcommand to the service.
    ///
    /// The `match` is exhaustive over the four real subcommands; the
    /// residual arm is a named error, never a panic (V-ERR-04).
    fn dispatch(&self, matches: &ArgMatches) -> Result<(), CliError> {
        let (name, sub) = matches
            .subcommand()
            .ok_or_else(|| CliError::MissingArgument("subcommand".to_owned()))?;
        match name {
            "add" => {
                // V-ERR-02: the legacy panic on a missing argument becomes a
                // let-else; the named error replaces the panic.
                let Some(cmd) = sub.get_one::<String>("COMMAND") else {
                    return Err(CliError::MissingArgument("COMMAND".to_owned()));
                };
                // ARCH-04: convert the raw CLI string at the boundary.
                let entry = QueueEntry::from_str(cmd).map_err(CliError::BadCommand)?;
                self.service.add(&entry)?;
                Ok(())
            }
            "start" => Ok(self.service.start()?),
            "ls" => {
                // Optional single `-n`/`--numbered` flag; the clap builder
                // rejects any other token with a usage error (exit 2) itself.
                let numbered = sub.get_flag("numbered");
                let entries = self.service.ls()?;
                if numbered {
                    print!("{}", ls_numbered(&entries));
                } else {
                    print!("{}", ls_output(&entries));
                }
                Ok(())
            }
            "rm" => {
                // A missing `N` is caught here as a named error rather than a
                // panic (V-ERR-02); clap also exits 2 on a missing required
                // arg, so this arm is the defensive, in-process path.
                let Some(n) = sub.get_one::<String>("N") else {
                    return Err(CliError::MissingArgument("N".to_owned()));
                };
                // ARCH-04: parse the raw index at the boundary into the
                // domain newtype; 0 / non-numeric / out-of-u32 is a
                // parse-time rejection (exit 2 via `BadCommand`).
                let index = QueueEntryIndex::from_str(n).map_err(CliError::BadCommand)?;
                match self.service.remove_entry(index.as_u32() as u64) {
                    Ok(()) => Ok(()),
                    // An in-range-parse but beyond-the-queue position is a
                    // usage-class failure (exit 2, message on stderr, queue
                    // unchanged) — the domain's `NoEntry` surfaced through
                    // the `BadCommand` arm (no new `CliError` variant).
                    Err(QueueError::NoEntry(pos)) => {
                        Err(CliError::BadCommand(QueueEntryError::IndexOutOfRange {
                            requested: pos,
                        }))
                    }
                    // Any other store failure (e.g. a missing queue file) is a
                    // service error (exit 1).
                    Err(e) => Err(CliError::Service(e)),
                }
            }
            "edit" => self
                .service
                .edit(&self.queue_path)
                .map_err(CliError::Service),
            // Exhaustive over the four real subcommands; this arm is a
            // defensive named error, NEVER a panic (V-ERR-04).
            other => Err(CliError::MissingArgument(other.to_owned())),
        }
    }
}

/// Build the clap `Command` with the legacy binary's exact configuration —
/// names, version, author, about strings, `subcommand_required(true)`, the
/// subcommands (`add`/`start`/`ls`/`rm`/`edit`) and the required positional
/// `COMMAND` argument — so help/usage/error output stays byte-identical to
/// the legacy binary.
fn command() -> Command {
    Command::new("Encode Queue")
        .version("1.0")
        .author("brianjsummerfield@gmail.com")
        .about("Manages a queue of commands")
        .subcommand_required(true)
        .subcommand(
            Command::new("add")
                .about("Adds a command to the queue")
                .arg(
                    Arg::new("COMMAND")
                        .help("The command to add")
                        .required(true)
                        .action(ArgAction::Set),
                ),
        )
        .subcommand(Command::new("start").about("Starts processing the queue"))
        .subcommand(
            Command::new("ls")
                .about("Lists all commands in the queue")
                .arg(
                    Arg::new("numbered")
                        .short('n')
                        .long("numbered")
                        .help("Prefix each line with its 1-based position")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("rm")
                .about("Removes the N-th command from the queue (1-based)")
                .arg(
                    Arg::new("N")
                        .help("The 1-based index of the command to remove")
                        .required(true)
                        .action(ArgAction::Set),
                ),
        )
        .subcommand(Command::new("edit").about("Opens the queue in an editor"))
}

/// Build the `ls` output: every entry verbatim, one per line, followed by a
/// final blank line — byte-identical to the legacy `println!("{}", contents)`
/// for any queue file (which this tool always writes with a trailing
/// newline, so legacy output == every entry + '\n', then one final '\n').
///
/// A queue file without a trailing newline is out of contract: this tool
/// never produces one (`add` appends via `writeln!`, `start` rewrites via
/// `writeln!`).
fn ls_output(entries: &[QueueEntry]) -> String {
    let mut out = String::new();
    for e in entries {
        out.push_str(e.as_str());
        out.push('\n');
    }
    out.push('\n');
    out
}

/// Build the numbered `ls` output (`ls -n`/`ls --numbered`): one line per
/// entry, `{pos}: {cmd}` with a 1-based position and the raw entry string
/// verbatim, followed by the same single final blank line as plain `ls`
/// (`ls_output`) — so an empty queue yields exactly one `"\n"` (the
/// 2026-10-16 empty-queue convention, identical to plain `ls`).
fn ls_numbered(entries: &[QueueEntry]) -> String {
    let mut out = String::new();
    for (i, e) in entries.iter().enumerate() {
        out.push_str(&(i + 1).to_string());
        out.push_str(": ");
        out.push_str(e.as_str());
        out.push('\n');
    }
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    // In-memory mock ports (TST-01): no real filesystem, no environment, no
    // `fish`/editor process is ever touched. Plain `Mutex`, no `Arc`
    // (GEN-02). Lock poisoning is mapped to the boundary catch-all variant
    // instead of `unwrap` (D6.1).

    use super::*;
    use crate::ports::command_runner::RunError;
    use crate::ports::editor::EditorError;
    use crate::ports::queue_store::QueueStoreError;
    use std::path::PathBuf;
    use std::sync::{Mutex, PoisonError};

    const TEST_QUEUE_FILE: &str = "/tmp/p5wp09-test/commands.txt";

    /// Test-side error type: the test functions return
    /// `Result<(), TestError>` so `?` works on clap parse results, service
    /// results and `QueueEntry` construction alike (clap's own `Error`
    /// builder API is feature-gated in the pinned version, so the tests
    /// convert each source into this named error instead).
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestError(String);

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&self.0)
        }
    }

    impl std::error::Error for TestError {}

    impl From<clap::Error> for TestError {
        fn from(e: clap::Error) -> Self {
            TestError(e.to_string())
        }
    }

    impl From<QueueError> for TestError {
        fn from(e: QueueError) -> Self {
            TestError(e.to_string())
        }
    }

    impl From<QueueEntryError> for TestError {
        fn from(e: QueueEntryError) -> Self {
            TestError(e.to_string())
        }
    }

    /// In-memory `QueueStore` mock: `entries` is the backing queue;
    /// `read_err` and `drop_err` are one-shot scripted failures (returned on
    /// the next matching call).
    struct MockStore {
        entries: Mutex<Vec<QueueEntry>>,
        read_err: Mutex<Option<QueueStoreError>>,
        drop_err: Mutex<Option<QueueStoreError>>,
    }

    fn poisoned_store_error<E>(_e: PoisonError<E>) -> QueueStoreError {
        QueueStoreError::Io(std::io::Error::other("test mutex poisoned"))
    }

    impl QueueStore for MockStore {
        fn read_all(&self) -> Result<Vec<QueueEntry>, QueueStoreError> {
            if let Some(err) = self.read_err.lock().map_err(poisoned_store_error)?.take() {
                return Err(err);
            }
            Ok(self.entries.lock().map_err(poisoned_store_error)?.clone())
        }

        fn append(&self, entry: &QueueEntry) -> Result<(), QueueStoreError> {
            self.entries
                .lock()
                .map_err(poisoned_store_error)?
                .push(entry.clone());
            Ok(())
        }

        fn drop_first(&self) -> Result<(), QueueStoreError> {
            if let Some(err) = self.drop_err.lock().map_err(poisoned_store_error)?.take() {
                return Err(err);
            }
            let mut entries = self.entries.lock().map_err(poisoned_store_error)?;
            if !entries.is_empty() {
                entries.remove(0);
            }
            Ok(())
        }

        fn write_all(&self, entries: &[QueueEntry]) -> Result<(), QueueStoreError> {
            *self.entries.lock().map_err(poisoned_store_error)? = entries.to_vec();
            Ok(())
        }
    }

    /// `CommandRunner` mock: no call recording (D6.1); returns the one-shot
    /// scripted `fail` error (consumed on the first call).
    struct MockRunner {
        fail: Mutex<Option<RunError>>,
    }

    fn poisoned_run_error<E>(_e: PoisonError<E>) -> RunError {
        RunError::Other(std::io::Error::other("test mutex poisoned"))
    }

    impl CommandRunner for MockRunner {
        fn run(&self, _entry: &QueueEntry) -> Result<(), RunError> {
            self.fail
                .lock()
                .map_err(poisoned_run_error)?
                .take()
                .map_or(Ok(()), Err)
        }
    }

    /// `EditorLauncher` mock: `expected = Some(p)` means `open` returns `Ok`
    /// only when `*at == p` (a path mismatch is an `Err`), so a successful
    /// `edit` proves the injected queue path was the one passed through;
    /// `fail` forces an `Err` regardless of the path. No real editor is
    /// spawned.
    struct MockEditor {
        expected: Option<QueuePath>,
        fail: bool,
    }

    impl EditorLauncher for MockEditor {
        fn open(&self, at: &QueuePath) -> Result<(), EditorError> {
            if self.fail {
                return Err(EditorError::Spawn(std::io::Error::other("editor failed")));
            }
            if self.expected.as_ref() == Some(at) {
                Ok(())
            } else {
                Err(EditorError::Other(std::io::Error::other(
                    "unexpected queue path",
                )))
            }
        }
    }

    /// Build a `QueueEntry` (the lenient P5 constructor — cannot fail).
    fn queue_entry(line: &str) -> Result<QueueEntry, QueueEntryError> {
        QueueEntry::from_str(line)
    }

    /// The injected test queue path; a literal that never touches the
    /// filesystem (TST-01).
    fn test_queue_path() -> QueuePath {
        QueuePath::new(PathBuf::from(TEST_QUEUE_FILE))
    }

    fn store(entries: Vec<QueueEntry>) -> MockStore {
        MockStore {
            entries: Mutex::new(entries),
            read_err: Mutex::new(None),
            drop_err: Mutex::new(None),
        }
    }

    fn runner(fail: Option<RunError>) -> MockRunner {
        MockRunner {
            fail: Mutex::new(fail),
        }
    }

    fn make_cli(
        store: MockStore,
        runner: MockRunner,
        editor: MockEditor,
    ) -> Cli<MockStore, MockRunner, MockEditor> {
        let service = QueueService::new(store, runner, editor);
        Cli::new(service, test_queue_path())
    }

    #[test]
    fn cli_add_appends_raw_argument_verbatim() -> Result<(), TestError> {
        let cli = make_cli(
            store(vec![queue_entry("seed")?]),
            runner(None),
            MockEditor {
                expected: None,
                fail: false,
            },
        );
        let matches = command().try_get_matches_from(["encode-queue", "add", "  nix build .  "])?;

        assert!(matches!(cli.dispatch(&matches), Ok(())));

        // Lenient boundary: the raw argument is enqueued verbatim, untrimmed.
        let lines: Vec<String> = cli
            .service
            .ls()?
            .iter()
            .map(|e| e.as_str().to_owned())
            .collect();
        assert_eq!(lines, vec!["seed".to_owned(), "  nix build .  ".to_owned()]);
        Ok(())
    }

    #[test]
    fn cli_start_drains_the_queue_through_the_service() -> Result<(), TestError> {
        let cli = make_cli(
            store(vec![queue_entry("one")?, queue_entry("two")?]),
            runner(None),
            MockEditor {
                expected: None,
                fail: false,
            },
        );
        let matches = command().try_get_matches_from(["encode-queue", "start"])?;

        assert!(matches!(cli.dispatch(&matches), Ok(())));
        assert!(cli.service.ls()?.is_empty());
        Ok(())
    }

    #[test]
    fn cli_ls_prints_entries_verbatim_plus_final_blank_line() {
        // Pure function: empty queue → exactly one blank line (the legacy
        // `println!` on an empty, trailing-newline-terminated file).
        assert_eq!(ls_output(&[]), "\n");

        // Blank/whitespace-only entries are printed verbatim.
        let entries: Vec<QueueEntry> =
            match vec![queue_entry("a"), queue_entry("   "), queue_entry("b")]
                .into_iter()
                .collect::<Result<Vec<QueueEntry>, QueueEntryError>>()
            {
                Ok(entries) => entries,
                Err(_) => return, // the lenient P5 constructor never fails
            };
        assert_eq!(ls_output(&entries), "a\n   \nb\n\n");

        let blank: QueueEntry = match queue_entry("  ") {
            Ok(entry) => entry,
            Err(_) => return, // the lenient P5 constructor never fails
        };
        assert_eq!(ls_output(&[blank]), "  \n\n");
    }

    #[test]
    fn cli_edit_opens_the_editor_on_the_injected_queue_path() -> Result<(), TestError> {
        let path = test_queue_path();
        let cli = make_cli(
            store(Vec::new()),
            runner(None),
            MockEditor {
                expected: Some(path.clone()),
                fail: false,
            },
        );
        let matches = command().try_get_matches_from(["encode-queue", "edit"])?;

        // The editor mock returns `Err` on any other path, so `Ok` proves
        // the injected `queue_path` was the one passed to the editor port.
        assert!(matches!(cli.dispatch(&matches), Ok(())));
        Ok(())
    }

    #[test]
    fn cli_maps_store_error_to_service_variant() -> Result<(), TestError> {
        // P6 (supervisor-approved test-only deviation): the one-shot store
        // error is the `Io` catch-all rather than `QueueFileMissing`, because
        // `ls` on a missing file now returns `Ok(empty)` (V-ARCH-05 behaviour
        // half, covered by the WP08 service test). An `Io` error is still a
        // legitimate `ls` failure, so the `CliError::Service` mapping stays
        // exercised at CLI level.
        let cli = make_cli(
            MockStore {
                entries: Mutex::new(Vec::new()),
                read_err: Mutex::new(Some(QueueStoreError::Io(std::io::Error::other(
                    "queue file unreadable",
                )))),
                drop_err: Mutex::new(None),
            },
            runner(None),
            MockEditor {
                expected: None,
                fail: false,
            },
        );
        let matches = command().try_get_matches_from(["encode-queue", "ls"])?;

        let err = cli.dispatch(&matches);
        assert!(matches!(
            err,
            Err(CliError::Service(QueueError::Store(QueueStoreError::Io(_))))
        ));
        Ok(())
    }

    #[test]
    fn cli_maps_runner_error_to_service_variant_and_keeps_the_line() -> Result<(), TestError> {
        let cli = make_cli(
            store(vec![queue_entry("cmd")?]),
            runner(Some(RunError::Other(std::io::Error::other("run failed")))),
            MockEditor {
                expected: None,
                fail: false,
            },
        );
        let matches = command().try_get_matches_from(["encode-queue", "start"])?;

        let err = cli.dispatch(&matches);
        assert!(matches!(err, Err(CliError::Service(QueueError::Run(_)))));

        // The line is only removed on success: the store still holds it.
        let lines: Vec<String> = cli
            .service
            .ls()?
            .iter()
            .map(|e| e.as_str().to_owned())
            .collect();
        assert_eq!(lines, vec!["cmd".to_owned()]);
        Ok(())
    }

    #[test]
    fn cli_maps_editor_error_to_service_variant() -> Result<(), TestError> {
        let cli = make_cli(
            store(Vec::new()),
            runner(None),
            MockEditor {
                expected: None,
                fail: true,
            },
        );
        let matches = command().try_get_matches_from(["encode-queue", "edit"])?;

        let err = cli.dispatch(&matches);
        assert!(matches!(err, Err(CliError::Service(QueueError::Editor(_)))));
        Ok(())
    }

    #[test]
    fn cli_rm_removes_the_target_entry() -> Result<(), TestError> {
        let cli = make_cli(
            store(vec![
                queue_entry("a")?,
                queue_entry("b")?,
                queue_entry("c")?,
            ]),
            runner(None),
            MockEditor {
                expected: None,
                fail: false,
            },
        );
        let matches = command().try_get_matches_from(["encode-queue", "rm", "2"])?;

        assert!(matches!(cli.dispatch(&matches), Ok(())));

        let lines: Vec<String> = cli
            .service
            .ls()?
            .iter()
            .map(|e| e.as_str().to_owned())
            .collect();
        assert_eq!(lines, vec!["a".to_owned(), "c".to_owned()]);
        Ok(())
    }

    #[test]
    fn cli_rm_rejects_zero_and_non_numeric_at_parse_time() -> Result<(), TestError> {
        for n in ["0", "abc"] {
            let cli = make_cli(
                store(vec![queue_entry("a")?]),
                runner(None),
                MockEditor {
                    expected: None,
                    fail: false,
                },
            );
            let matches = command().try_get_matches_from(["encode-queue", "rm", n])?;

            // Parse-time rejection: `InvalidIndex`, not a service error.
            let err = cli.dispatch(&matches);
            assert!(
                matches!(
                    err,
                    Err(CliError::BadCommand(QueueEntryError::InvalidIndex { .. }))
                ),
                "n={n}"
            );
            // The queue is unchanged.
            assert_eq!(cli.service.ls()?.len(), 1);
        }
        Ok(())
    }

    #[test]
    fn cli_rm_maps_index_out_of_range_to_bad_command() -> Result<(), TestError> {
        let cli = make_cli(
            store(vec![queue_entry("a")?, queue_entry("b")?]),
            runner(None),
            MockEditor {
                expected: None,
                fail: false,
            },
        );
        let matches = command().try_get_matches_from(["encode-queue", "rm", "4"])?;

        let err = cli.dispatch(&matches);
        assert!(matches!(
            err,
            Err(CliError::BadCommand(
                QueueEntryError::IndexOutOfRange { .. }
            ))
        ));

        // The queue is left unchanged.
        assert_eq!(cli.service.ls()?.len(), 2);
        Ok(())
    }

    #[test]
    fn cli_ls_numbered_prints_position_prefix() {
        // Empty queue: exactly one "\n" (the 2026-10-16 convention, same as
        // plain `ls`).
        assert_eq!(ls_numbered(&[]), "\n");

        let entries: Vec<QueueEntry> =
            match vec![queue_entry("a"), queue_entry("b"), queue_entry("c")]
                .into_iter()
                .collect::<Result<Vec<QueueEntry>, QueueEntryError>>()
            {
                Ok(entries) => entries,
                Err(_) => return, // the lenient P5 constructor never fails
            };
        assert_eq!(ls_numbered(&entries), "1: a\n2: b\n3: c\n\n");
    }
}
