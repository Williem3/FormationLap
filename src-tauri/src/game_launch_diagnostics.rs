use crate::{CoreError, GameLaunchDiagnostic, atomic_file::replace_with_backup};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub(crate) struct GameLaunchDiagnostics {
    backup_path: PathBuf,
    report_path: PathBuf,
}

impl GameLaunchDiagnostics {
    pub(crate) fn open(storage_root: impl AsRef<Path>) -> Result<Self, CoreError> {
        let storage_root = storage_root.as_ref();
        let logs_directory = storage_root.join("logs");
        let backups_directory = storage_root.join("backups");
        fs::create_dir_all(&logs_directory)?;
        fs::create_dir_all(&backups_directory)?;
        let report_path = logs_directory.join("test-game-launch.json");
        let backup_path = backups_directory.join("test-game-launch.json");
        let temporary_path = report_path.with_extension("json.tmp");
        if temporary_path.exists() {
            fs::remove_file(temporary_path)?;
        }
        Ok(Self {
            backup_path,
            report_path,
        })
    }

    pub(crate) fn persist(&self, diagnostic: &GameLaunchDiagnostic) -> Result<(), CoreError> {
        let temporary_path = self.report_path.with_extension("json.tmp");
        if temporary_path.exists() {
            fs::remove_file(&temporary_path)?;
        }
        let mut serialized = serde_json::to_vec_pretty(diagnostic)
            .map_err(CoreError::InvalidGameLaunchDiagnostic)?;
        serialized.push(b'\n');
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)?;
        file.write_all(&serialized)?;
        file.sync_all()?;
        drop(file);

        if self.report_path.exists() {
            replace_with_backup(&self.report_path, &temporary_path, &self.backup_path)?;
        } else {
            fs::rename(&temporary_path, &self.report_path)?;
        }
        Ok(())
    }
}
