mod adapters;
mod domain;
mod ports;
mod usecases;

use clap::{Arg, ArgAction, Command};
use std::{env, fs, io};

/// CT002→CT003 staging seam: the binary-edge error for the intermediate state.
///
/// CT003 replaces this with the public `CliError` (`src/error.rs`), adds the
/// `main()`/`run()` split, and explicit exit codes. For now it keeps `main()`
/// concrete (no trait object — P-disp) and removes the three CT002 panics.
#[derive(thiserror::Error)]
enum RunError {
    /// A domain conversion/config error.
    #[error(transparent)]
    Domain(#[from] domain::error::DomainError),

    /// An I/O error from a use case or `create_dir_all`.
    #[error(transparent)]
    Io(#[from] io::Error),

    /// A required argument was somehow missing (defensive; `required(true)`
    /// makes this unreachable after a successful parse).
    #[error("missing required argument: COMMAND")]
    MissingCommand,
}

// `Termination for Result` (main -> Result) prints the error with `{:?}` —
// Debug, not Display. Format Debug via Display so users see the typed
// message (plan §5), e.g. `Error: HOME environment variable is not set`.
impl std::fmt::Debug for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

fn main() -> Result<(), RunError> {
    let matches = Command::new("Encode Queue")
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
                        .action(ArgAction::Set)
                        // CT002: validate at the boundary (C012/P-hex). A
                        // blank/invalid value is rejected by clap as a usage
                        // error (exit 2) before the domain is reached.
                        .value_parser(clap::builder::ValueParser::new(
                            |s: &str| domain::entry::QueueEntry::try_from(s),
                        )),
                ),
        )
        .subcommand(Command::new("start").about("Starts processing the queue"))
        .subcommand(Command::new("ls").about("Lists all commands in the queue"))
        .subcommand(Command::new("edit").about("Opens the queue in an editor"))
        .get_matches();

    // CT002: typed config boundary (was `env::var("HOME").expect(...)`).
    let home = env::var_os("HOME").ok_or(RunError::Domain(
        domain::error::DomainError::HomeMissing,
    ))?;
    let queue = domain::config::QueuePath::try_from(home)?;

    // Runs for every subcommand (incl. `edit`) — preserved exactly.
    fs::create_dir_all(queue.dir())?;

    match matches.subcommand() {
        Some(("add", sub_matches)) => {
            let command = sub_matches
                .get_one::<domain::entry::QueueEntry>("COMMAND")
                .ok_or(RunError::MissingCommand)?; // was .unwrap()
            usecases::add_queue_entry::AddQueueEntry::new(
                adapters::file_queue_repo::FileQueueRepo::new(queue.as_path().to_path_buf()),
            )
            .execute(command)?;
        }
        Some(("start", _)) => {
            usecases::process_queue::ProcessQueue::new(
                adapters::file_queue_repo::FileQueueRepo::new(queue.as_path().to_path_buf()),
                adapters::fish_command_runner::FishCommandRunner,
            )
            .execute()?;
        }
        Some(("ls", _)) => {
            usecases::list_queue::ListQueue::new(
                adapters::file_queue_repo::FileQueueRepo::new(queue.as_path().to_path_buf()),
            )
            .execute()?;
        }
        Some(("edit", _)) => {
            usecases::edit_queue::EditQueue::new(queue.clone()).execute()?;
        }
        _ => unreachable!(), // CT003 owns this (public CliError / run split).
    }

    Ok(())
}
