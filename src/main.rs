mod adapters;
mod domain;
mod ports;
mod usecases;

use clap::{Arg, ArgAction, Command};
use std::{env, fs, io, path::PathBuf};

fn main() -> io::Result<()> {
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
                        .action(ArgAction::Set),
                ),
        )
        .subcommand(Command::new("start").about("Starts processing the queue"))
        .subcommand(Command::new("ls").about("Lists all commands in the queue"))
        .subcommand(Command::new("edit").about("Opens the queue in an editor"))
        .get_matches();

    let home_dir = env::var("HOME").expect("HOME environment variable not set");
    let queue_dir = PathBuf::from(home_dir).join(".config/encode_queue");
    let queue_path = queue_dir.join("commands.txt");

    // Runs for every subcommand (incl. `edit`) — preserved exactly.
    fs::create_dir_all(&queue_dir)?;

    match matches.subcommand() {
        Some(("add", sub_matches)) => {
            let command = sub_matches.get_one::<String>("COMMAND").unwrap();
            usecases::add_queue_entry::AddQueueEntry::new(
                adapters::file_queue_repo::FileQueueRepo::new(queue_path),
            )
            .execute(&domain::entry::QueueEntry::new(command))?;
        }
        Some(("start", _)) => {
            usecases::process_queue::ProcessQueue::new(
                adapters::file_queue_repo::FileQueueRepo::new(queue_path),
                adapters::fish_command_runner::FishCommandRunner,
            )
            .execute()?;
        }
        Some(("ls", _)) => {
            usecases::list_queue::ListQueue::new(
                adapters::file_queue_repo::FileQueueRepo::new(queue_path),
            )
            .execute()?;
        }
        Some(("edit", _)) => {
            usecases::edit_queue::EditQueue::new(queue_path).execute()?;
        }
        _ => unreachable!(),
    }

    Ok(())
}
