//! `encode_queue` CLI bootstrap (ARCH-02): config → outbound adapters →
//! service → inbound CLI → exit code (0 ok, 1 service/I-O, 2 usage, 3 lock
//! held).
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
#![deny(clippy::inherent_to_string_shadow_display)]

mod config;
mod domain;
mod inbound;
mod outbound;
mod ports;

use std::process::ExitCode;

use crate::config::{Config, ConfigError};
use crate::domain::queue::service::QueueService;
use crate::inbound::cli::{Cli, CliError};
use crate::outbound::editor::EditorProcess;
use crate::outbound::lock::FileQueueLock;
use crate::outbound::queue_file::FileQueueStore;
use crate::outbound::shell::ShellRunner;

fn main() -> ExitCode {
    let config = match Config::from_env() {
        Ok(config) => config,
        // P6.F2: an invalid shell-selection value is a usage-class error
        // and exits with code 2, distinct from the generic config-error arm.
        Err(e) if matches!(e, ConfigError::ShellInvalid(_)) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };

    // P7.FIX03: the store owns the queue directory's creation (ARCH-02:
    // `main` does no file I/O; ARCH-03: all file I/O in `outbound/`;
    // behaviour contract: "dir created if missing" for every command).
    // A mkdir failure still exits 1 before dispatch, as before.
    let store = match FileQueueStore::new(config.queue_path().clone()) {
        Ok(store) => store,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };

    // `QueuePath` is not `Copy` (TY-06: `PathBuf` wrapper), so the lock
    // takes a clone, exactly like the store (P6.F4).
    let service = QueueService::new(
        store,
        ShellRunner::from(*config.shell()),
        EditorProcess::new(config.editor().clone()),
        FileQueueLock::new(config.queue_path().clone()),
    );
    let cli = Cli::new(service, config.queue_path().clone());

    match cli.run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::BadCommand(e)) => {
            eprintln!("invalid command: {e}");
            ExitCode::from(2)
        }
        Err(CliError::MissingArgument(arg)) => {
            eprintln!("error: missing required argument: {arg}");
            ExitCode::from(2)
        }
        // P6.F4: another live process holds the start lock: a distinct,
        // non-failure exit code (3). Placed BEFORE the `Service` catch-all.
        Err(CliError::Locked { pid }) => {
            eprintln!("error: queue already running (pid {pid})");
            ExitCode::from(3)
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
