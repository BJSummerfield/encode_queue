
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuePath(String);

impl QueuePath {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }

    pub fn dir(&self) -> &Path {
        Path::new(&self.0).parent().unwrap_or_else(|| Path::new(&self.0))
    }
}

impl TryFrom<OsString> for QueuePath {
    type Error = DomainError;

    fn try_from(home: OsString) -> Result<Self, Self::Error> {
        let home = home.into_string().map_err(DomainError::HomeNotUtf8)?;
        let file = PathBuf::from(home)
            .join(".config/encode_queue/commands.txt")
            .into_os_string();
        let file = file.into_string().map_err(DomainError::QueuePathNotUtf8)?;
        Ok(Self(file))
    }
}

impl std::fmt::Display for QueuePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::convert::AsRef<str> for QueuePath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Editor(String);

impl Editor {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<OsString> for Editor {
    type Error = DomainError;

    fn try_from(value: OsString) -> Result<Self, Self::Error> {
        let s = value.into_string().map_err(DomainError::EditorNotUtf8)?;
        Ok(Self(s))
    }
}

impl std::fmt::Display for Editor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::convert::AsRef<str> for Editor {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
