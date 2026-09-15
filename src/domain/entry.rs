//! Domain value: one queue entry (a fish command line).

/// One queue entry — a single fish command line, stored **unmodified** (no
/// trim/escape). `TryFrom<&str>` is the sole validating construction path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEntry(String);

impl QueueEntry {
    /// The raw command line exactly as validated and stored.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for QueueEntry {
    type Error = crate::domain::error::DomainError;

    /// Boundary conversion (C012/P-hex): reject empty/whitespace-only values,
    /// otherwise store the value verbatim.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            return Err(crate::domain::error::DomainError::EmptyQueueEntry(
                value.to_string(),
            ));
        }
        Ok(Self(value.to_string()))
    }
}
