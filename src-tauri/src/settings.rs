use crate::CoreError;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsDocument {
    schema_version: u32,
    selected_profile_id: Option<String>,
}

pub(crate) struct SettingsStore {
    backup_path: PathBuf,
    document: SettingsDocument,
    settings_path: PathBuf,
}

impl SettingsStore {
    pub(crate) fn open(storage_root: impl AsRef<Path>) -> Result<Self, CoreError> {
        let storage_root = storage_root.as_ref();
        let backups_directory = storage_root.join("backups");
        fs::create_dir_all(&backups_directory)?;
        let settings_path = storage_root.join("settings.json");
        let backup_path = backups_directory.join("settings.json");
        let document = if settings_path.exists() {
            let document: SettingsDocument = serde_json::from_slice(&fs::read(&settings_path)?)
                .map_err(CoreError::InvalidSettingsDocument)?;
            if document.schema_version != SETTINGS_SCHEMA_VERSION {
                return Err(CoreError::UnsupportedSettingsSchema(
                    document.schema_version,
                ));
            }
            document
        } else {
            SettingsDocument {
                schema_version: SETTINGS_SCHEMA_VERSION,
                selected_profile_id: None,
            }
        };

        Ok(Self {
            backup_path,
            document,
            settings_path,
        })
    }

    pub(crate) fn selected_profile_id(&self) -> Option<&str> {
        self.document.selected_profile_id.as_deref()
    }

    pub(crate) fn select_profile(&mut self, profile_id: String) -> Result<(), CoreError> {
        let document = SettingsDocument {
            schema_version: SETTINGS_SCHEMA_VERSION,
            selected_profile_id: Some(profile_id),
        };
        let temporary = self.settings_path.with_extension("json.tmp");
        let mut serialized =
            serde_json::to_vec_pretty(&document).map_err(CoreError::InvalidSettingsDocument)?;
        serialized.push(b'\n');
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&serialized)?;
        file.sync_all()?;
        drop(file);

        if self.settings_path.exists() {
            if self.backup_path.exists() {
                fs::remove_file(&self.backup_path)?;
            }
            fs::rename(&self.settings_path, &self.backup_path)?;
        }
        if let Err(error) = fs::rename(&temporary, &self.settings_path) {
            if self.backup_path.exists() {
                let _ = fs::rename(&self.backup_path, &self.settings_path);
            }
            return Err(error.into());
        }

        self.document = document;
        Ok(())
    }
}
