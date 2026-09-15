use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::domain::entry::QueueEntry;
use crate::ports::queue_repo::QueueRepo;

/// File-backed queue: one entry per line at the configured path
/// (`~/.config/encode_queue/commands.txt`), created by `main`'s
/// `create_dir_all` exactly as today.
pub struct FileQueueRepo {
    path: PathBuf,
}

impl FileQueueRepo {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl QueueRepo for FileQueueRepo {
    fn read_entries(&self) -> std::io::Result<Vec<QueueEntry>> {
        let file = File::open(&self.path)?;
        let lines = BufReader::new(file).lines();
        let owned: std::io::Result<Vec<String>> = lines.collect();
        // CT002: convert each line at the read boundary via TryFrom (C012/P-hex).
        // Only blank/whitespace-only lines fail validation; they are skipped (not
        // stored as entries), preserving CT001's "skip blank line, no spawn" drain.
        // `.ok()` is safe: the only possible error is EmptyQueueEntry.
        let entries = owned?
            .into_iter()
            .filter_map(|line| QueueEntry::try_from(line.as_str()).ok())
            .collect();
        Ok(entries)
    }

    fn append_entry(&self, entry: &QueueEntry) -> std::io::Result<()> {
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        writeln!(file, "{}", entry.as_str())?;
        Ok(())
    }

    fn replace_with_remaining(&self, remaining: &[QueueEntry]) -> std::io::Result<()> {
        // write+truncate (no create), as the current truncate branch.
        let mut file = OpenOptions::new().write(true).truncate(true).open(&self.path)?;
        for line in remaining {
            writeln!(file, "{}", line.as_str())?;
        }
        Ok(())
    }
}
