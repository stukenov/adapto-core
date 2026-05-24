use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::StoreError;

// ---------------------------------------------------------------------------
// WAL entry types
// ---------------------------------------------------------------------------

/// A single entry in the write-ahead log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalEntry {
    Insert {
        collection: String,
        doc_id: String,
        data: Value,
        tenant_id: Option<String>,
    },
    Update {
        collection: String,
        doc_id: String,
        data: Value,
    },
    Delete {
        collection: String,
        doc_id: String,
    },
    CreateCollection {
        name: String,
    },
    DropCollection {
        name: String,
    },
    CreateIndex {
        collection: String,
        fields: Vec<String>,
        unique: bool,
    },
    Snapshot {
        data: Value,
    },
}

// ---------------------------------------------------------------------------
// WriteAheadLog
// ---------------------------------------------------------------------------

/// Append-only write-ahead log backed by a single file.
///
/// Each entry is serialised as one JSON line. On startup, the log is replayed
/// to reconstruct in-memory state. Compaction writes a `Snapshot` entry
/// containing the full state and truncates everything before it.
pub struct WriteAheadLog {
    path: PathBuf,
    file: File,
}

impl WriteAheadLog {
    /// Open (or create) a WAL file at the given path.
    pub fn open(path: &str) -> Result<Self, StoreError> {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;
        Ok(Self { path, file })
    }

    /// Append an entry to the log (one JSON line, flushed immediately).
    pub fn append(&mut self, entry: &WalEntry) -> Result<(), StoreError> {
        let line = serde_json::to_string(entry)
            .map_err(|e| StoreError::Serialization(e.to_string()))?;
        writeln!(self.file, "{}", line)?;
        Ok(())
    }

    /// Read all entries from the log, starting from the last snapshot (if any).
    pub fn replay(&self) -> Result<Vec<WalEntry>, StoreError> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);

        let mut all_entries = Vec::new();
        let mut last_snapshot_idx: Option<usize> = None;

        for (i, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: WalEntry = serde_json::from_str(&line)
                .map_err(|e| StoreError::WalCorrupted(format!("line {}: {}", i + 1, e)))?;
            if matches!(entry, WalEntry::Snapshot { .. }) {
                last_snapshot_idx = Some(all_entries.len());
            }
            all_entries.push(entry);
        }

        // If there is a snapshot, return only the snapshot and entries after it.
        if let Some(idx) = last_snapshot_idx {
            Ok(all_entries.into_iter().skip(idx).collect())
        } else {
            Ok(all_entries)
        }
    }

    /// Write a snapshot entry and truncate the log to only that entry.
    pub fn compact(&mut self, snapshot: Value) -> Result<(), StoreError> {
        // Write the full state as a single snapshot entry to a new file,
        // then atomically replace the old one.
        let tmp_path = self.path.with_extension("wal.tmp");
        {
            let mut tmp = File::create(&tmp_path)?;
            let entry = WalEntry::Snapshot { data: snapshot };
            let line = serde_json::to_string(&entry)
                .map_err(|e| StoreError::Serialization(e.to_string()))?;
            writeln!(tmp, "{}", line)?;
            tmp.sync_all()?;
        }
        fs::rename(&tmp_path, &self.path)?;

        // Re-open the file handle in append mode.
        self.file = OpenOptions::new()
            .append(true)
            .open(&self.path)?;
        Ok(())
    }

    /// Current size of the WAL file in bytes.
    pub fn size_bytes(&self) -> u64 {
        fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0)
    }
}
