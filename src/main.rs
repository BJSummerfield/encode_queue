mod adapters;
mod domain;
mod error;
mod ports;
mod usecases;

use clap::{Arg, ArgAction, Command};
use std::{env, fs, process};

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        process::exit(1);
    }
}

fn run() -> Result<(), error::CliError> {
    let matches = Command::new("Encode Queue")
        .version(env!("CARGO_PKG_VERSION"))
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
                        .value_parser(clap::builder::ValueParser::new(
                            |s: &str| domain::entry::QueueEntry::try_from(s),
                        )),
                ),
        )
        .subcommand(Command::new("start").about("Starts processing the queue"))
        .subcommand(Command::new("ls").about("Lists all commands in the queue"))
        .subcommand(Command::new("edit").about("Opens the queue in an editor"))
        .get_matches();

    let home = env::var_os("HOME")
        .ok_or(domain::error::DomainError::HomeMissing)
        .map_err(|_| error::CliError::ConfigHome)?;
    let queue = domain::config::QueuePath::try_from(home)
        .map_err(|_| error::CliError::ConfigHome)?;

    fs::create_dir_all(queue.dir()).map_err(|source| error::CliError::QueueWrite {
        source,
        path: queue.dir().to_path_buf(),
    })?;

    match matches.subcommand() {
        Some(("add", sub_matches)) => {
            let command = sub_matches
                .get_one::<domain::entry::QueueEntry>("COMMAND")
                .ok_or(error::CliError::MissingCommand)?;
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
        _ => {
            eprintln!("error: a subcommand is required (add, start, ls or edit)");
            eprintln!("run 'encode_queue --help' for usage");
            process::exit(2);
        }
    }

    Ok(())
}
