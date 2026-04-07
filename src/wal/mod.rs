pub mod event;

use anyhow::Result;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use self::event::WalEntry;

/// Config / test override: if set, data lives under this directory (e.g. temp dir in tests).
pub fn taskwal_dir() -> PathBuf {
    std::env::var("TASKWAL_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .expect("home directory not found")
                .join(".taskwal")
        })
}

pub fn wal_path() -> PathBuf {
    taskwal_dir().join("wal.log")
}

pub fn append_to(path: &std::path::Path, entry: &WalEntry) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let line = serde_json::to_string(entry)?;
    writeln!(file, "{}", line)?;
    file.sync_all()?;
    Ok(())
}

pub fn append(entry: &WalEntry) -> Result<()> {
    append_to(&wal_path(), entry)
}

pub fn read_all_from(path: &std::path::Path) -> Result<Vec<WalEntry>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut entries = vec![];
    for (line_no, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<WalEntry>(&line) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                eprintln!(
                    "taskwal: skipping malformed WAL line {}: {}",
                    line_no + 1,
                    e
                );
            }
        }
    }
    Ok(entries)
}

pub fn read_all() -> Result<Vec<WalEntry>> {
    read_all_from(&wal_path())
}

#[cfg(test)]
mod tests {
    use super::event::{Column, WalEntry, WalEvent};
    use super::*;
    use chrono::Utc;
    use tempfile::tempdir;

    #[test]
    fn append_read_roundtrip() -> Result<()> {
        let dir = tempdir()?;
        let log = dir.path().join("wal.log");
        let entry = WalEntry {
            ts: Utc::now(),
            event: WalEvent::Add {
                id: "01TESTTESTTESTTESTTEST".to_string(),
                title: "hello".to_string(),
                tags: vec![],
                day: "2026-04-07".to_string(),
            },
        };
        append_to(&log, &entry)?;
        let read = read_all_from(&log)?;
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].event, entry.event);
        Ok(())
    }

    #[test]
    fn move_event_serializes() -> Result<()> {
        let dir = tempdir()?;
        let log = dir.path().join("wal.log");
        let e = WalEntry {
            ts: Utc::now(),
            event: WalEvent::Move {
                id: "01TESTTESTTESTTESTTEST".to_string(),
                to: Column::Doing,
            },
        };
        append_to(&log, &e)?;
        let line = std::fs::read_to_string(&log)?;
        let parsed: WalEntry = serde_json::from_str(line.trim())?;
        assert!(matches!(parsed.event, WalEvent::Move { .. }));
        Ok(())
    }
}
