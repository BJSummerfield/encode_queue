use crate::domain::{QueueError, QueueRepository};

pub fn list_commands<Q: QueueRepository>(repo: &Q) -> Result<(), QueueError> {
    let commands = repo.list()?;
    for (i, cmd) in commands.iter().enumerate() {
        println!("{}: {}", i + 1, cmd.as_str());
    }
    Ok(())
}