use clap::Parser;
use encode_queue::{
    adapters::FileQueue,
    domain::QueueError,
    features,
};

#[derive(Parser)]
#[command(name = "encode_queue", version = "1.0", about = "Manages a queue of commands")]
enum Cli {
    /// Adds a command to the queue
    Add {
        /// The command to add
        command: String,
    },
    /// Starts processing the queue
    Start,
    /// Lists all commands in the queue
    Ls,
    /// Opens the queue in an editor
    Edit,
}

fn main() -> Result<(), QueueError> {
    let cli = Cli::parse();
    let mut repo = FileQueue::new()?;

    match cli {
        Cli::Add { command } => features::add::add_command(&mut repo, &command)?,
        Cli::Start => features::process::process_queue(&mut repo)?,
        Cli::Ls => features::list::list_commands(&repo)?,
        Cli::Edit => features::edit::edit_queue(&repo)?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_commands_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn cli_requires_subcommand() {
        let result = Cli::try_parse_from(["encode_queue"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_add_parses_command() {
        let cli = Cli::try_parse_from(["encode_queue", "add", "echo hello"]).unwrap();
        assert!(matches!(cli, Cli::Add { command } if command == "echo hello"));
    }

    #[test]
    fn cli_start_parses() {
        let cli = Cli::try_parse_from(["encode_queue", "start"]).unwrap();
        assert!(matches!(cli, Cli::Start));
    }

    #[test]
    fn cli_ls_parses() {
        let cli = Cli::try_parse_from(["encode_queue", "ls"]).unwrap();
        assert!(matches!(cli, Cli::Ls));
    }

    #[test]
    fn cli_edit_parses() {
        let cli = Cli::try_parse_from(["encode_queue", "edit"]).unwrap();
        assert!(matches!(cli, Cli::Edit));
    }
}
