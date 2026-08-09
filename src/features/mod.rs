pub mod add;
pub mod edit;
pub mod list;
pub mod process;

#[cfg(test)]
mod tests {
    use crate::domain::{CommandText, QueueError, QueueRepository};

    struct MockRepo {
        commands: Vec<CommandText>,
    }

    impl QueueRepository for MockRepo {
        fn add(&mut self, command: &CommandText) -> Result<(), QueueError> {
            self.commands.push(command.clone());
            Ok(())
        }
        fn list(&self) -> Result<Vec<CommandText>, QueueError> {
            Ok(self.commands.clone())
        }
        fn get_and_remove_first(&mut self) -> Result<Option<CommandText>, QueueError> {
            if self.commands.is_empty() {
                Ok(None)
            } else {
                Ok(Some(self.commands.remove(0)))
            }
        }
        fn open_in_editor(&self) -> Result<(), QueueError> {
            Ok(())
        }
    }

    #[test]
    fn add_command_valid() {
        let mut repo = MockRepo { commands: vec![] };
        super::add::add_command(&mut repo, "echo hello").unwrap();
        assert_eq!(repo.commands.len(), 1);
        assert_eq!(repo.commands[0].as_str(), "echo hello");
    }

    #[test]
    fn add_command_empty_rejected() {
        let mut repo = MockRepo { commands: vec![] };
        assert!(super::add::add_command(&mut repo, "").is_err());
    }

    #[test]
    fn add_command_whitespace_only_rejected() {
        let mut repo = MockRepo { commands: vec![] };
        assert!(super::add::add_command(&mut repo, "   ").is_err());
    }

    #[test]
    fn list_commands_returns_ok() {
        let repo = MockRepo {
            commands: vec![CommandText::new("cmd1").unwrap()],
        };
        assert!(super::list::list_commands(&repo).is_ok());
    }

    #[test]
    fn edit_queue_returns_ok() {
        let repo = MockRepo { commands: vec![] };
        assert!(super::edit::edit_queue(&repo).is_ok());
    }
}
