use crate::domain::{QueueError, QueueRepository};

pub fn edit_queue<Q: QueueRepository>(repo: &Q) -> Result<(), QueueError> {
    repo.open_in_editor()
}
