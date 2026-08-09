use crate::domain::{CommandText, QueueError, QueueRepository};

pub fn add_command<Q: QueueRepository>(repo: &mut Q, raw: &str) -> Result<(), QueueError> {
    let command = CommandText::new(raw)?;
    repo.add(&command)
}