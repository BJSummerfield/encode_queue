
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEntry(String);

impl QueueEntry {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for QueueEntry {
    type Error = crate::domain::error::DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err(crate::domain::error::DomainError::EmptyQueueEntry(
                value.to_string(),
            ));
        }
        Ok(Self(value.to_string()))
    }
}

impl std::fmt::Display for QueueEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::convert::AsRef<str> for QueueEntry {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
