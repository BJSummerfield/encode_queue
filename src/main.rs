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

use std::fs;
use std::process::ExitCode;

use crate::config::{Config, ConfigError};
use crate::domain::queue::service::QueueService;
use crate::inbound::cli::{Cli, CliError};
use crate::outbound::editor::EditorProcess;
use crate::outbound::queue_file::FileQueueStore;
use crate::outbound::shell::ShellRunner;

fn main() -> ExitCode {
    let config = match Config::from_env() {
        Ok(config) => config,
        // Usage-class error (P6.F2): an invalid shell-selection value is
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

    // Legacy bootstrap parity (behaviour contract): the queue directory is
    // created before any subcommand dispatches, so `add` works on a fresh
    // `$HOME` while a missing queue file still surfaces as
    // `QueueFileMissing` for `ls`/`start`.
    if let Some(queue_dir) = config.queue_path().as_path().parent() {
        if let Err(e) = fs::create_dir_all(queue_dir) {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    }

    let service = QueueService::new(
        FileQueueStore::new(config.queue_path().clone()),
        ShellRunner::from(*config.shell()),
        EditorProcess::new(config.editor().clone()),
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
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
