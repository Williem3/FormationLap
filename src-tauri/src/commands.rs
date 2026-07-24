use crate::{
    AppCommand, AppSnapshot, CommandOutcome, CoreError, DiscoverySnapshot, FormationLapCore,
    RacingProfile, SupportingApplicationRecommendation, TargetedDiscoverySources,
};
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

/// Racing Profile and configured application accepted by start commands.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ApplicationTargetPayload {
    pub profile_id: String,
    pub application_id: String,
}

/// Explicit application exit request plus Pre-existing Process confirmation.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ExitApplicationPayload {
    pub application_id: String,
    pub pre_existing_confirmed: bool,
}

/// Explicit force-stop request with both required ownership confirmations.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ForceStopApplicationPayload {
    pub application_id: String,
    pub pre_existing_confirmed: bool,
    pub force_confirmed: bool,
}

/// Explicit restart request plus Pre-existing Process confirmation.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct RestartApplicationPayload {
    pub profile_id: String,
    pub application_id: String,
    pub pre_existing_confirmed: bool,
}

/// Curated Primary Sim target accepted by recommendation commands.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PrimarySimIdPayload {
    pub primary_sim_id: String,
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
            CoreError::DiscoveryCatalog(_) => (
                "invalid_curated_catalog",
                "Formation Lap could not open its bundled Curated Catalog.".to_owned(),
                Some("Reinstall Formation Lap from an official signed release."),
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

    pub fn open_with_runtime(
        storage_root: impl AsRef<Path>,
        process_runtime: impl crate::ProcessRuntime + 'static,
    ) -> Result<Self, CommandError> {
        Ok(Self {
            core: Mutex::new(
                FormationLapCore::open_with_runtime(storage_root, process_runtime)
                    .map_err(CommandError::from)?,
            ),
        })
    }

    pub fn open_with_discovery_sources(
        storage_root: impl AsRef<Path>,
        discovery_sources: TargetedDiscoverySources,
    ) -> Result<Self, CommandError> {
        Ok(Self {
            core: Mutex::new(
                FormationLapCore::open_with_discovery_sources(storage_root, discovery_sources)
                    .map_err(CommandError::from)?,
            ),
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

    pub fn start_application(
        &self,
        payload: ApplicationTargetPayload,
    ) -> Result<AppSnapshot, CommandError> {
        let mut core = self.core()?;
        core.execute(AppCommand::StartApplication {
            profile_id: payload.profile_id,
            application_id: payload.application_id,
        })
        .map_err(CommandError::from)?;
        Ok(core.snapshot())
    }

    pub fn refresh_processes(&self) -> Result<AppSnapshot, CommandError> {
        let mut core = self.core()?;
        core.execute(AppCommand::RefreshProcesses)
            .map_err(CommandError::from)?;
        Ok(core.snapshot())
    }

    pub fn exit_application(
        &self,
        payload: ExitApplicationPayload,
    ) -> Result<AppSnapshot, CommandError> {
        let mut core = self.core()?;
        core.execute(AppCommand::ExitApplication {
            application_id: payload.application_id,
            pre_existing_confirmed: payload.pre_existing_confirmed,
        })
        .map_err(CommandError::from)?;
        Ok(core.snapshot())
    }

    pub fn force_stop_application(
        &self,
        payload: ForceStopApplicationPayload,
    ) -> Result<AppSnapshot, CommandError> {
        let mut core = self.core()?;
        core.execute(AppCommand::ForceStopApplication {
            application_id: payload.application_id,
            pre_existing_confirmed: payload.pre_existing_confirmed,
            force_confirmed: payload.force_confirmed,
        })
        .map_err(CommandError::from)?;
        Ok(core.snapshot())
    }

    pub fn restart_application(
        &self,
        payload: RestartApplicationPayload,
    ) -> Result<AppSnapshot, CommandError> {
        let mut core = self.core()?;
        core.execute(AppCommand::RestartApplication {
            profile_id: payload.profile_id,
            application_id: payload.application_id,
            pre_existing_confirmed: payload.pre_existing_confirmed,
        })
        .map_err(CommandError::from)?;
        Ok(core.snapshot())
    }

    pub fn discover_applications(&self) -> Result<DiscoverySnapshot, CommandError> {
        let mut core = self.core()?;
        match core
            .execute(AppCommand::DiscoverApplications)
            .map_err(CommandError::from)?
        {
            CommandOutcome::ApplicationsDiscovered { discovery } => Ok(discovery),
            _ => Err(CommandError {
                code: "unexpected_outcome".to_owned(),
                message: "Formation Lap could not complete local discovery.".to_owned(),
                recovery: Some("Try discovery again.".to_owned()),
                diagnostic_id: None,
            }),
        }
    }

    pub fn recommend_applications(
        &self,
        payload: PrimarySimIdPayload,
    ) -> Result<Vec<SupportingApplicationRecommendation>, CommandError> {
        let mut core = self.core()?;
        match core
            .execute(AppCommand::RecommendApplications {
                primary_sim_id: payload.primary_sim_id,
            })
            .map_err(CommandError::from)?
        {
            CommandOutcome::ApplicationsRecommended { recommendations } => Ok(recommendations),
            _ => Err(CommandError {
                code: "unexpected_outcome".to_owned(),
                message: "Formation Lap could not rank application recommendations.".to_owned(),
                recovery: Some("Select the Primary Sim again.".to_owned()),
                diagnostic_id: None,
            }),
        }
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

#[tauri::command]
pub fn start_application(
    commands: tauri::State<'_, NativeCommandHost>,
    payload: ApplicationTargetPayload,
) -> Result<AppSnapshot, CommandError> {
    commands.start_application(payload)
}

#[tauri::command]
pub fn refresh_processes(
    commands: tauri::State<'_, NativeCommandHost>,
) -> Result<AppSnapshot, CommandError> {
    commands.refresh_processes()
}

#[tauri::command]
pub fn exit_application(
    commands: tauri::State<'_, NativeCommandHost>,
    payload: ExitApplicationPayload,
) -> Result<AppSnapshot, CommandError> {
    commands.exit_application(payload)
}

#[tauri::command]
pub fn force_stop_application(
    commands: tauri::State<'_, NativeCommandHost>,
    payload: ForceStopApplicationPayload,
) -> Result<AppSnapshot, CommandError> {
    commands.force_stop_application(payload)
}

#[tauri::command]
pub fn restart_application(
    commands: tauri::State<'_, NativeCommandHost>,
    payload: RestartApplicationPayload,
) -> Result<AppSnapshot, CommandError> {
    commands.restart_application(payload)
}

#[tauri::command]
pub fn discover_applications(
    commands: tauri::State<'_, NativeCommandHost>,
) -> Result<DiscoverySnapshot, CommandError> {
    commands.discover_applications()
}

#[tauri::command]
pub fn recommend_applications(
    commands: tauri::State<'_, NativeCommandHost>,
    payload: PrimarySimIdPayload,
) -> Result<Vec<SupportingApplicationRecommendation>, CommandError> {
    commands.recommend_applications(payload)
}
