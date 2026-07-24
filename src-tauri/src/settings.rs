use crate::{CoreError, atomic_file::replace_with_backup};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsDocument {
    schema_version: u32,
    selected_profile_id: Option<String>,
    #[serde(default)]
    desktop: crate::DesktopSettings,
    #[serde(default)]
    last_automatic_update_check_unix_seconds: Option<u64>,
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
        let temporary_path = settings_path.with_extension("json.tmp");
        if temporary_path.exists() {
            if settings_path.exists() {
                fs::remove_file(&temporary_path)?;
            } else if backup_path.exists() {
                let recovered: SettingsDocument = serde_json::from_slice(&fs::read(&backup_path)?)
                    .map_err(CoreError::InvalidSettingsDocument)?;
                if recovered.schema_version != SETTINGS_SCHEMA_VERSION {
                    return Err(CoreError::UnsupportedSettingsSchema(
                        recovered.schema_version,
                    ));
                }
                fs::remove_file(&temporary_path)?;
                fs::rename(&backup_path, &settings_path)?;
            } else {
                fs::remove_file(&temporary_path)?;
            }
        }
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
                desktop: crate::DesktopSettings::default(),
                last_automatic_update_check_unix_seconds: None,
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

    pub(crate) fn desktop(&self) -> &crate::DesktopSettings {
        &self.document.desktop
    }

    pub(crate) fn last_automatic_update_check_unix_seconds(&self) -> Option<u64> {
        self.document.last_automatic_update_check_unix_seconds
    }

    pub(crate) fn select_profile(&mut self, profile_id: String) -> Result<(), CoreError> {
        let mut document = self.document.clone();
        document.selected_profile_id = Some(profile_id);
        self.persist(document)
    }

    pub(crate) fn update_desktop(
        &mut self,
        desktop: crate::DesktopSettings,
    ) -> Result<(), CoreError> {
        let mut document = self.document.clone();
        document.desktop = desktop;
        self.persist(document)
    }

    pub(crate) fn record_automatic_update_check(
        &mut self,
        unix_seconds: u64,
    ) -> Result<(), CoreError> {
        let mut document = self.document.clone();
        document.last_automatic_update_check_unix_seconds = Some(unix_seconds);
        self.persist(document)
    }

    fn persist(&mut self, document: SettingsDocument) -> Result<(), CoreError> {
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
            replace_with_backup(&self.settings_path, &temporary, &self.backup_path)?;
        } else {
            fs::rename(&temporary, &self.settings_path)?;
        }

        self.document = document;
        Ok(())
    }
}
