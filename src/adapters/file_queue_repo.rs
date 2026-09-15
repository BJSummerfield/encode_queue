use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::domain::entry::QueueEntry;
use crate::error::CliError;
use crate::ports::queue_repo::QueueRepo;

pub struct FileQueueRepo {
    path: PathBuf,
}

impl FileQueueRepo {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl QueueRepo for FileQueueRepo {
    fn read_entries(&self) -> Result<Vec<QueueEntry>, CliError> {
        let file = File::open(&self.path).map_err(|source| CliError::QueueRead {
            source,
            path: self.path.clone(),
        })?;
        let lines = BufReader::new(file).lines();
        let owned: std::io::Result<Vec<String>> = lines.collect();
        let owned = owned.map_err(|source| CliError::QueueRead {
            source,
            path: self.path.clone(),
        })?;
        let entries = owned
            .into_iter()
            .filter_map(|line| QueueEntry::try_from(line.as_str()).ok())
            .collect();
        Ok(entries)
    }

    fn append_entry(&self, entry: &QueueEntry) -> Result<(), CliError> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|source| CliError::QueueWrite {
                source,
                path: self.path.clone(),
            })?;
        writeln!(file, "{}", entry.as_str()).map_err(|source| CliError::QueueWrite {
            source,
            path: self.path.clone(),
        })?;
        Ok(())
    }

    fn replace_with_remaining(&self, remaining: &[QueueEntry]) -> Result<(), CliError> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|source| CliError::QueueWrite {
                source,
                path: self.path.clone(),
            })?;
        for line in remaining {
            writeln!(file, "{}", line.as_str()).map_err(|source| CliError::QueueWrite {
                source,
                path: self.path.clone(),
            })?;
        }
        Ok(())
    }
}
