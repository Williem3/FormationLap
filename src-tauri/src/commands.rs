use crate::{AppCommand, AppSnapshot, CommandOutcome, CoreError, FormationLapCore, RacingProfile};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Mutex, MutexGuard},
};
use ts_rs::TS;

/// Typed input accepted by the narrow create-profile command.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CreateProfilePayload {
    pub name: String,
    pub primary_sim_name: String,
}

/// Complete editable Racing Profile accepted by the save command.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SaveProfilePayload {
    pub profile: RacingProfile,
}

/// Stable Racing Profile target accepted by selection and destructive commands.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ProfileIdPayload {
    pub profile_id: String,
}

/// Source and new name accepted by the duplicate-profile command.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DuplicateProfilePayload {
    pub source_profile_id: String,
    pub name: String,
}

/// Portable JSON accepted by the import-profile command.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ImportProfilePayload {
    pub document: String,
}

/// Structured error returned across the Rust/TypeScript seam.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
    pub recovery: Option<String>,
    pub diagnostic_id: Option<String>,
}

impl From<CoreError> for CommandError {
    fn from(error: CoreError) -> Self {
        let (code, message, recovery) = match error {
            CoreError::InvalidProfileName(_) => (
                "invalid_profile",
                error.to_string(),
                Some("Enter a name and try again."),
            ),
            CoreError::ProfileNotFound(_) => (
                "profile_not_found",
                error.to_string(),
                Some("Refresh the profile list and try again."),
            ),
            CoreError::ApplicationNotFound(_) => (
                "application_not_found",
                error.to_string(),
                Some("Refresh the Racing Profile and try again."),
            ),
            CoreError::ProcessRuntime(_) => (
                "process_runtime_failed",
                error.to_string(),
                Some("Check the application path and try again."),
            ),
            CoreError::Storage(_) => (
                "storage_failed",
                "Formation Lap could not update local profile storage.".to_owned(),
                Some("Check local storage access and try again."),
            ),
            CoreError::InvalidProfileDocument(_)
            | CoreError::UnsupportedProfileSchema(_)
            | CoreError::InvalidSettingsDocument(_)
            | CoreError::UnsupportedSettingsSchema(_) => (
                "invalid_local_state",
                "Formation Lap found local profile data it cannot safely open.".to_owned(),
                Some("Restore a valid backup or export diagnostics."),
            ),
        };

        Self {
            code: code.to_owned(),
            message,
            recovery: recovery.map(str::to_owned),
            diagnostic_id: None,
        }
    }
}

/// Owns one serialized FormationLapCore instance for all native commands.
pub struct NativeCommandHost {
    core: Mutex<FormationLapCore>,
}

impl NativeCommandHost {
    pub fn open(storage_root: impl AsRef<Path>) -> Result<Self, CommandError> {
        Ok(Self {
            core: Mutex::new(FormationLapCore::open(storage_root).map_err(CommandError::from)?),
        })
    }

    fn core(&self) -> Result<MutexGuard<'_, FormationLapCore>, CommandError> {
        self.core.lock().map_err(|_| CommandError {
            code: "core_unavailable".to_owned(),
            message: "Formation Lap could not access its authoritative state.".to_owned(),
            recovery: Some("Close and reopen Formation Lap.".to_owned()),
            diagnostic_id: None,
        })
    }

    pub fn get_app_snapshot(&self) -> Result<AppSnapshot, CommandError> {
        Ok(self.core()?.snapshot())
    }

    pub fn create_profile(
        &self,
        payload: CreateProfilePayload,
    ) -> Result<AppSnapshot, CommandError> {
        let mut core = self.core()?;
        core.execute(AppCommand::CreateProfile {
            name: payload.name,
            primary_sim_name: payload.primary_sim_name,
        })
        .map_err(CommandError::from)?;
        Ok(core.snapshot())
    }

    pub fn save_profile(&self, payload: SaveProfilePayload) -> Result<AppSnapshot, CommandError> {
        let mut core = self.core()?;
        core.execute(AppCommand::SaveProfile {
            profile: Box::new(payload.profile),
        })
        .map_err(CommandError::from)?;
        Ok(core.snapshot())
    }

    pub fn select_profile(&self, payload: ProfileIdPayload) -> Result<AppSnapshot, CommandError> {
        let mut core = self.core()?;
        core.execute(AppCommand::SelectProfile {
            profile_id: payload.profile_id,
        })
        .map_err(CommandError::from)?;
        Ok(core.snapshot())
    }

    pub fn duplicate_profile(
        &self,
        payload: DuplicateProfilePayload,
    ) -> Result<AppSnapshot, CommandError> {
        let mut core = self.core()?;
        core.execute(AppCommand::DuplicateProfile {
            source_profile_id: payload.source_profile_id,
            name: payload.name,
        })
        .map_err(CommandError::from)?;
        Ok(core.snapshot())
    }

    pub fn delete_profile(&self, payload: ProfileIdPayload) -> Result<AppSnapshot, CommandError> {
        let mut core = self.core()?;
        core.execute(AppCommand::DeleteProfile {
            profile_id: payload.profile_id,
        })
        .map_err(CommandError::from)?;
        Ok(core.snapshot())
    }

    pub fn export_profile(&self, payload: ProfileIdPayload) -> Result<String, CommandError> {
        let mut core = self.core()?;
        match core
            .execute(AppCommand::ExportProfile {
                profile_id: payload.profile_id,
            })
            .map_err(CommandError::from)?
        {
            CommandOutcome::ProfileExported { document } => Ok(document),
            _ => Err(CommandError {
                code: "unexpected_outcome".to_owned(),
                message: "Formation Lap could not complete the profile export.".to_owned(),
                recovery: Some("Try the export again.".to_owned()),
                diagnostic_id: None,
            }),
        }
    }

    pub fn import_profile(
        &self,
        payload: ImportProfilePayload,
    ) -> Result<AppSnapshot, CommandError> {
        let mut core = self.core()?;
        core.execute(AppCommand::ImportProfile {
            document: payload.document,
        })
        .map_err(CommandError::from)?;
        Ok(core.snapshot())
    }
}

#[tauri::command]
pub fn get_app_snapshot(
    commands: tauri::State<'_, NativeCommandHost>,
) -> Result<AppSnapshot, CommandError> {
    commands.get_app_snapshot()
}

#[tauri::command]
pub fn create_profile(
    commands: tauri::State<'_, NativeCommandHost>,
    payload: CreateProfilePayload,
) -> Result<AppSnapshot, CommandError> {
    commands.create_profile(payload)
}

#[tauri::command]
pub fn save_profile(
    commands: tauri::State<'_, NativeCommandHost>,
    payload: SaveProfilePayload,
) -> Result<AppSnapshot, CommandError> {
    commands.save_profile(payload)
}

#[tauri::command]
pub fn select_profile(
    commands: tauri::State<'_, NativeCommandHost>,
    payload: ProfileIdPayload,
) -> Result<AppSnapshot, CommandError> {
    commands.select_profile(payload)
}

#[tauri::command]
pub fn duplicate_profile(
    commands: tauri::State<'_, NativeCommandHost>,
    payload: DuplicateProfilePayload,
) -> Result<AppSnapshot, CommandError> {
    commands.duplicate_profile(payload)
}

#[tauri::command]
pub fn delete_profile(
    commands: tauri::State<'_, NativeCommandHost>,
    payload: ProfileIdPayload,
) -> Result<AppSnapshot, CommandError> {
    commands.delete_profile(payload)
}

#[tauri::command]
pub fn export_profile(
    commands: tauri::State<'_, NativeCommandHost>,
    payload: ProfileIdPayload,
) -> Result<String, CommandError> {
    commands.export_profile(payload)
}

#[tauri::command]
pub fn import_profile(
    commands: tauri::State<'_, NativeCommandHost>,
    payload: ImportProfilePayload,
) -> Result<AppSnapshot, CommandError> {
    commands.import_profile(payload)
}
