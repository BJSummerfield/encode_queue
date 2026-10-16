//! Domain newtype for the shell the queue runner spawns (P6.F2, OP4).
//!
//! `Shell` is the boundary parse target for the shell-selection environment
//! variable: `config.rs` converts the raw env value into a `Shell` (or a
//! `ShellError`) and the outbound `ShellRunner` maps it to the program it
//! spawns. Case-sensitive, exact match only (OP4: parse, don't validate).

/// A shell the queue runner can spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    /// `fish`.
    Fish,
    /// `bash`.
    Bash,
    /// `sh`.
    Sh,
}

/// A value that is not a known shell name.
#[derive(Debug, thiserror::Error)]
#[error("'{input}' is not a valid shell: expected fish, bash, or sh")]
pub struct ShellError {
    /// The offending input (owned, ERR-04).
    pub input: String,
}

impl Shell {
    /// Parse a shell name. Case-sensitive, exact match only.
    pub fn new(input: &str) -> Result<Self, ShellError> {
        match input {
            "fish" => Ok(Shell::Fish),
            "bash" => Ok(Shell::Bash),
            "sh" => Ok(Shell::Sh),
            other => Err(ShellError {
                input: other.to_owned(),
            }),
        }
    }

    /// The program name to spawn.
    pub fn program(&self) -> &'static str {
        match self {
            Shell::Fish => "fish",
            Shell::Bash => "bash",
            Shell::Sh => "sh",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each valid name parses to the matching variant.
    #[test]
    fn new_parses_each_valid_name() {
        assert_eq!(Shell::new("fish").unwrap(), Shell::Fish);
        assert_eq!(Shell::new("bash").unwrap(), Shell::Bash);
        assert_eq!(Shell::new("sh").unwrap(), Shell::Sh);
    }

    /// An unknown name is rejected and the error carries the input.
    #[test]
    fn new_rejects_unknown_name_with_input() {
        let err = Shell::new("zsh").unwrap_err();
        assert_eq!(err.input, "zsh");
    }

    /// Matching is case-sensitive: `Fish` is not `fish`.
    #[test]
    fn new_rejects_wrong_case() {
        let err = Shell::new("Fish").unwrap_err();
        assert_eq!(err.input, "Fish");
    }

    /// `program` names the shell binary verbatim.
    #[test]
    fn program_names_the_shell_binary() {
        assert_eq!(Shell::Fish.program(), "fish");
        assert_eq!(Shell::Bash.program(), "bash");
        assert_eq!(Shell::Sh.program(), "sh");
    }
}
