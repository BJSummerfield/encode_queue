/// One line in the queue of commands to execute.
///
/// CT001 placeholder: infallible constructor holding the line exactly as
/// read. CT002 introduces the validating `TryFrom` conversion at the edges.
#[derive(Debug, Clone)]
pub struct QueueEntry(String);

impl QueueEntry {
    /// Wrap a raw queue line (unmodified; no trim/escaping — fish parses it).
    pub fn new(line: impl Into<String>) -> Self {
        Self(line.into())
    }

    /// The line content without the trailing newline.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
