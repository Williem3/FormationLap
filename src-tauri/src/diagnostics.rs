use crate::{CoreError, DiagnosticEntry};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_LOG_BYTES: u64 = 65_536;
const MAX_EXPORT_ENTRIES: usize = 128;

pub(crate) struct DiagnosticLog {
    live_path: PathBuf,
    rotated_path: PathBuf,
}

impl DiagnosticLog {
    pub(crate) fn open(storage_root: impl AsRef<Path>) -> Result<Self, CoreError> {
        let directory = storage_root.as_ref().join("logs");
        fs::create_dir_all(&directory)?;
        Ok(Self {
            live_path: directory.join("formation-lap.jsonl"),
            rotated_path: directory.join("formation-lap.1.jsonl"),
        })
    }

    pub(crate) fn record(&self, event: &str, outcome: &str) -> Result<(), CoreError> {
        let entry = DiagnosticEntry {
            timestamp_unix_seconds: unix_seconds(),
            event: event.to_owned(),
            outcome: outcome.to_owned(),
        };
        let mut line =
            serde_json::to_vec(&entry).expect("DiagnosticEntry serialization cannot fail");
        line.push(b'\n');

        let current_length = self
            .live_path
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or_default();
        if current_length.saturating_add(line.len() as u64) > MAX_LOG_BYTES {
            if self.rotated_path.exists() {
                fs::remove_file(&self.rotated_path)?;
            }
            if self.live_path.exists() {
                fs::rename(&self.live_path, &self.rotated_path)?;
            }
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.live_path)?;
        file.write_all(&line)?;
        file.flush()?;
        Ok(())
    }

    pub(crate) fn recent_entries(&self) -> Vec<DiagnosticEntry> {
        let mut entries = Vec::new();
        for path in [&self.rotated_path, &self.live_path] {
            let Ok(document) = fs::read_to_string(path) else {
                continue;
            };
            entries.extend(
                document
                    .lines()
                    .filter_map(|line| serde_json::from_str::<DiagnosticEntry>(line).ok()),
            );
        }
        let remove = entries.len().saturating_sub(MAX_EXPORT_ENTRIES);
        entries.drain(..remove);
        entries
    }
}

pub(crate) fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
