use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandText(String);

impl CommandText {
    pub fn new(raw: &str) -> Result<Self, InvalidCommandError> {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            return Err(InvalidCommandError::Empty);
        }
        Ok(Self(trimmed))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'a> TryFrom<&'a str> for CommandText {
    type Error = InvalidCommandError;
    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Error, Debug)]
pub enum InvalidCommandError {
    #[error("command cannot be empty")]
    Empty,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_command() {
        let cmd = CommandText::new("echo hello").unwrap();
        assert_eq!(cmd.as_str(), "echo hello");
    }

    #[test]
    fn empty_command_rejected() {
        assert!(CommandText::new("").is_err());
    }

    #[test]
    fn whitespace_only_rejected() {
        assert!(CommandText::new("   ").is_err());
    }

    #[test]
    fn trims_whitespace() {
        let cmd = CommandText::new("  echo hello  ").unwrap();
        assert_eq!(cmd.as_str(), "echo hello");
    }

    #[test]
    fn try_from_str() {
        let cmd: CommandText = "echo hello".try_into().unwrap();
        assert_eq!(cmd.as_str(), "echo hello");
    }

    #[test]
    fn try_from_empty_str_fails() {
        let result: Result<CommandText, _> = ("").try_into();
        assert!(result.is_err());
    }
}
