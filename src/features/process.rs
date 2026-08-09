use std::process::{Command as SystemCommand, Stdio};

use crate::domain::{QueueError, QueueRepository};

pub fn process_queue<Q: QueueRepository>(repo: &mut Q) -> Result<(), QueueError> {
    loop {
        match repo.get_and_remove_first()? {
            Some(command) => {
                let mut child = SystemCommand::new("fish")
                    .arg("-c")
                    .arg(command.as_str())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()?;
                child.wait()?;
            }
            None => break,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::domain::{CommandText, QueueError, QueueRepository};

    struct CountingMock {
        pub calls: usize,
        remaining: bool,
    }

    impl QueueRepository for CountingMock {
        fn add(&mut self, _: &CommandText) -> Result<(), QueueError> {
            unimplemented!()
        }
        fn list(&self) -> Result<Vec<CommandText>, QueueError> {
            unimplemented!()
        }
        fn get_and_remove_first(&mut self) -> Result<Option<CommandText>, QueueError> {
            self.calls += 1;
            if self.remaining {
                self.remaining = false;
                Ok(Some(CommandText::new("true").unwrap()))
            } else {
                Ok(None)
            }
        }
        fn open_in_editor(&self) -> Result<(), QueueError> {
            unimplemented!()
        }
    }

    #[test]
    fn process_queue_stops_when_empty() {
        let mut repo = CountingMock { calls: 0, remaining: false };
        super::process_queue(&mut repo).unwrap();
        assert_eq!(repo.calls, 1);
    }

    #[test]
    fn process_queue_processes_one_command_then_stops() {
        let mut repo = CountingMock { calls: 0, remaining: true };
        super::process_queue(&mut repo).unwrap();
        assert_eq!(repo.calls, 2);
    }
}