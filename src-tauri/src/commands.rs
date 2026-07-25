use crate::{
    AppCommand, AppSnapshot, CommandOutcome, CoreError, DesktopSettings, DiagnosticExport,
    DirectUpdateProviderRuntime, DiscoverySnapshot, FormationLapCore, FormationLapInstallDecision,
    FormationLapUpdater, GameLaunchDiagnostic, QuitAction, QuitDisposition, RacingProfile,
    SupportingApplicationRecommendation, TargetedDiscoverySources, UpdateCheckDecision,
    UpdateCheckResult, UpdateCheckTrigger, UpdateProviderRunner, UpdateStatus, WindowCloseAction,
    update_coordinator::UpdateCoordinator,
};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::mpsc, thread};
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

/// Explicit executable-configuration approval for one reviewed Racing Profile.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ApproveProfilePayload {
    pub profile_id: String,
    pub configuration_reviewed: bool,
    pub approved_privileged_application_ids: Vec<String>,
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

/// Complete local desktop settings accepted by the update command.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UpdateSettingsPayload {
    pub settings: DesktopSettings,
}

/// Explicit application disposition accepted by the native Quit command.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct QuitPayload {
    pub disposition: QuitDisposition,
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
            CoreError::ProfileNeedsReview(_) | CoreError::InvalidProfileApproval(_) => (
                "profile_needs_review",
                error.to_string(),
                Some("Review every executable setting and approve privileged entries."),
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
            CoreError::PrivilegeBroker(_) => (
                "privileged_operation_failed",
                error.to_string(),
                Some("Approve the Windows prompt and verify the application path."),
            ),
            CoreError::DiscoveryCatalog(_) => (
                "invalid_curated_catalog",
                "Formation Lap could not open its bundled Curated Catalog.".to_owned(),
                Some("Reinstall Formation Lap from an official signed release."),
            ),
            CoreError::InvalidLaunchRecipe(_) => (
                "invalid_launch_recipe",
                error.to_string(),
                Some("Review the Primary Sim Launch Recipe and try again."),
            ),
            CoreError::InvalidSessionTransition { .. } => (
                "invalid_session_transition",
                error.to_string(),
                Some("Wait for the current Session action to finish."),
            ),
            CoreError::InvalidUpdateCheck(_) => (
                "invalid_update_check",
                error.to_string(),
                Some("Start a fresh update check and try again."),
            ),
            CoreError::ActivityConflict { .. } => (
                "activity_conflict",
                error.to_string(),
                Some("Wait for the current native activity to finish."),
            ),
            CoreError::Storage(ref storage_error)
                if storage_error.kind() == std::io::ErrorKind::InvalidData =>
            {
                (
                    "invalid_local_state",
                    "Formation Lap found local profile data it cannot safely open.".to_owned(),
                    Some("Restore a valid backup or export diagnostics."),
                )
            }
            CoreError::Storage(_) => (
                "storage_failed",
                "Formation Lap could not update local profile storage.".to_owned(),
                Some("Check local storage access and try again."),
            ),
            CoreError::InvalidProfileDocument(_)
            | CoreError::UnsupportedProfileSchema(_)
            | CoreError::InvalidSettingsDocument(_)
            | CoreError::UnsupportedSettingsSchema(_)
            | CoreError::InvalidSessionJournal(_)
            | CoreError::InvalidGameLaunchDiagnostic(_)
            | CoreError::UnsupportedSessionJournalSchema(_) => (
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

type WorkerResponse = Result<(CommandOutcome, AppSnapshot), CommandError>;

enum NativeWorkerRequest {
    Snapshot(mpsc::Sender<Result<AppSnapshot, CommandError>>),
    Execute {
        command: AppCommand,
        response: mpsc::Sender<WorkerResponse>,
    },
}

/// Sends every native request through one background FormationLapCore command loop.
#[derive(Clone)]
pub struct NativeCommandHost {
    sender: mpsc::Sender<NativeWorkerRequest>,
}

impl NativeCommandHost {
    pub fn open(storage_root: impl AsRef<Path>) -> Result<Self, CommandError> {
        Self::from_core(FormationLapCore::open(storage_root).map_err(CommandError::from)?)
    }

    pub fn open_with_roaming_migration(
        local_storage_root: impl AsRef<Path>,
        roaming_storage_root: impl AsRef<Path>,
    ) -> Result<Self, CommandError> {
        let local_storage_root = local_storage_root.as_ref();
        crate::storage_migration::prepare_local_storage(
            local_storage_root,
            roaming_storage_root.as_ref(),
        )
        .map_err(CommandError::from)?;
        Self::open(local_storage_root)
    }

    pub fn open_with_runtime(
        storage_root: impl AsRef<Path>,
        process_runtime: impl crate::ProcessRuntime + 'static,
    ) -> Result<Self, CommandError> {
        Self::from_core(
            FormationLapCore::open_with_runtime(storage_root, process_runtime)
                .map_err(CommandError::from)?,
        )
    }

    pub fn open_with_discovery_sources(
        storage_root: impl AsRef<Path>,
        discovery_sources: TargetedDiscoverySources,
    ) -> Result<Self, CommandError> {
        Self::from_core(
            FormationLapCore::open_with_discovery_sources(storage_root, discovery_sources)
                .map_err(CommandError::from)?,
        )
    }

    fn from_core(mut core: FormationLapCore) -> Result<Self, CommandError> {
        let (sender, receiver) = mpsc::channel();
        thread::Builder::new()
            .name("formation-lap-core".to_owned())
            .spawn(move || {
                while let Ok(request) = receiver.recv() {
                    match request {
                        NativeWorkerRequest::Snapshot(response) => {
                            let _ = response.send(Ok(core.snapshot()));
                        }
                        NativeWorkerRequest::Execute { command, response } => {
                            let result = core
                                .execute(command)
                                .map(|outcome| (outcome, core.snapshot()))
                                .map_err(CommandError::from);
                            let _ = response.send(result);
                        }
                    }
                }
            })
            .map_err(|_| Self::worker_unavailable())?;
        Ok(Self { sender })
    }

    fn worker_unavailable() -> CommandError {
        CommandError {
            code: "core_unavailable".to_owned(),
            message: "Formation Lap could not access its authoritative state.".to_owned(),
            recovery: Some("Close and reopen Formation Lap.".to_owned()),
            diagnostic_id: None,
        }
    }

    fn execute_command(&self, command: AppCommand) -> WorkerResponse {
        let (response, receiver) = mpsc::channel();
        self.sender
            .send(NativeWorkerRequest::Execute { command, response })
            .map_err(|_| Self::worker_unavailable())?;
        receiver.recv().map_err(|_| Self::worker_unavailable())?
    }

    pub fn get_app_snapshot(&self) -> Result<AppSnapshot, CommandError> {
        let (response, receiver) = mpsc::channel();
        self.sender
            .send(NativeWorkerRequest::Snapshot(response))
            .map_err(|_| Self::worker_unavailable())?;
        receiver.recv().map_err(|_| Self::worker_unavailable())?
    }

    pub fn create_profile(
        &self,
        payload: CreateProfilePayload,
    ) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::CreateProfile {
            name: payload.name,
            primary_sim_name: payload.primary_sim_name,
        })
        .map(|(_, snapshot)| snapshot)
    }

    pub fn save_profile(&self, payload: SaveProfilePayload) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::SaveProfile {
            profile: Box::new(payload.profile),
        })
        .map(|(_, snapshot)| snapshot)
    }

    pub fn select_profile(&self, payload: ProfileIdPayload) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::SelectProfile {
            profile_id: payload.profile_id,
        })
        .map(|(_, snapshot)| snapshot)
    }

    pub fn duplicate_profile(
        &self,
        payload: DuplicateProfilePayload,
    ) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::DuplicateProfile {
            source_profile_id: payload.source_profile_id,
            name: payload.name,
        })
        .map(|(_, snapshot)| snapshot)
    }

    pub fn delete_profile(&self, payload: ProfileIdPayload) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::DeleteProfile {
            profile_id: payload.profile_id,
        })
        .map(|(_, snapshot)| snapshot)
    }

    pub fn export_profile(&self, payload: ProfileIdPayload) -> Result<String, CommandError> {
        match self
            .execute_command(AppCommand::ExportProfile {
                profile_id: payload.profile_id,
            })?
            .0
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
        self.execute_command(AppCommand::ImportProfile {
            document: payload.document,
        })
        .map(|(_, snapshot)| snapshot)
    }

    pub fn approve_profile(
        &self,
        payload: ApproveProfilePayload,
    ) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::ApproveProfile {
            profile_id: payload.profile_id,
            configuration_reviewed: payload.configuration_reviewed,
            approved_privileged_application_ids: payload.approved_privileged_application_ids,
        })
        .map(|(_, snapshot)| snapshot)
    }

    pub fn start_application(
        &self,
        payload: ApplicationTargetPayload,
    ) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::StartApplication {
            profile_id: payload.profile_id,
            application_id: payload.application_id,
        })
        .map(|(_, snapshot)| snapshot)
    }

    pub fn refresh_processes(&self) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::RefreshProcesses)
            .map(|(_, snapshot)| snapshot)
    }

    pub fn exit_application(
        &self,
        payload: ExitApplicationPayload,
    ) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::ExitApplication {
            application_id: payload.application_id,
            pre_existing_confirmed: payload.pre_existing_confirmed,
        })
        .map(|(_, snapshot)| snapshot)
    }

    pub fn force_stop_application(
        &self,
        payload: ForceStopApplicationPayload,
    ) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::ForceStopApplication {
            application_id: payload.application_id,
            pre_existing_confirmed: payload.pre_existing_confirmed,
            force_confirmed: payload.force_confirmed,
        })
        .map(|(_, snapshot)| snapshot)
    }

    pub fn restart_application(
        &self,
        payload: RestartApplicationPayload,
    ) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::RestartApplication {
            profile_id: payload.profile_id,
            application_id: payload.application_id,
            pre_existing_confirmed: payload.pre_existing_confirmed,
        })
        .map(|(_, snapshot)| snapshot)
    }

    pub fn start_session(&self, payload: ProfileIdPayload) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::StartSession {
            profile_id: payload.profile_id,
        })
        .map(|(_, snapshot)| snapshot)
    }

    pub fn test_game_launch(
        &self,
        payload: ProfileIdPayload,
    ) -> Result<GameLaunchDiagnostic, CommandError> {
        match self
            .execute_command(AppCommand::TestGameLaunch {
                profile_id: payload.profile_id,
            })?
            .0
        {
            CommandOutcome::GameLaunchTested { diagnostic } => Ok(diagnostic),
            _ => Err(CommandError {
                code: "unexpected_outcome".to_owned(),
                message: "Formation Lap could not complete Test Game Launch.".to_owned(),
                recovery: Some("Try the launch test again.".to_owned()),
                diagnostic_id: None,
            }),
        }
    }

    pub fn cancel_startup(&self) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::CancelStartup)
            .map(|(_, snapshot)| snapshot)
    }

    pub fn close_session(&self) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::CloseSession)
            .map(|(_, snapshot)| snapshot)
    }

    pub fn request_window_close(&self) -> Result<WindowCloseAction, CommandError> {
        match self.execute_command(AppCommand::RequestWindowClose)?.0 {
            CommandOutcome::WindowCloseRequested { action } => Ok(action),
            _ => Err(Self::unexpected_outcome(
                "Formation Lap could not apply its window-close policy.",
            )),
        }
    }

    pub fn request_quit(
        &self,
        payload: QuitPayload,
    ) -> Result<(QuitAction, AppSnapshot), CommandError> {
        let (outcome, snapshot) = self.execute_command(AppCommand::RequestQuit {
            disposition: payload.disposition,
        })?;
        match outcome {
            CommandOutcome::QuitRequested { action } => Ok((action, snapshot)),
            _ => Err(Self::unexpected_outcome(
                "Formation Lap could not apply its Quit policy.",
            )),
        }
    }

    pub fn update_settings(
        &self,
        payload: UpdateSettingsPayload,
    ) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::UpdateSettings {
            settings: payload.settings,
        })
        .map(|(_, snapshot)| snapshot)
    }

    pub fn export_diagnostics(&self) -> Result<DiagnosticExport, CommandError> {
        match self.execute_command(AppCommand::ExportDiagnostics)?.0 {
            CommandOutcome::DiagnosticsExported { diagnostics } => Ok(diagnostics),
            _ => Err(Self::unexpected_outcome(
                "Formation Lap could not export local diagnostics.",
            )),
        }
    }

    pub fn prepare_update_check(
        &self,
        trigger: UpdateCheckTrigger,
    ) -> Result<UpdateCheckDecision, CommandError> {
        let now_unix_seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| CommandError {
                code: "invalid_system_clock".to_owned(),
                message: "Formation Lap could not read the system clock.".to_owned(),
                recovery: Some("Correct the Windows clock and try again.".to_owned()),
                diagnostic_id: None,
            })?
            .as_secs();
        match self
            .execute_command(AppCommand::PrepareUpdateCheck {
                trigger,
                now_unix_seconds,
            })?
            .0
        {
            CommandOutcome::UpdateCheckPrepared { decision } => Ok(decision),
            _ => Err(Self::unexpected_outcome(
                "Formation Lap could not prepare the update check.",
            )),
        }
    }

    pub fn complete_update_check(
        &self,
        result: UpdateCheckResult,
    ) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::CompleteUpdateCheck { result })
            .map(|(_, snapshot)| snapshot)
    }

    fn cancel_update_check(&self, request_id: String) -> Result<AppSnapshot, CommandError> {
        match self
            .execute_command(AppCommand::CancelUpdateCheck { request_id })?
            .0
        {
            CommandOutcome::UpdateCheckCancelled => self.get_app_snapshot(),
            _ => Err(Self::unexpected_outcome(
                "Formation Lap could not cancel the update check.",
            )),
        }
    }

    pub fn prepare_formation_lap_install(
        &self,
    ) -> Result<FormationLapInstallDecision, CommandError> {
        match self
            .execute_command(AppCommand::PrepareFormationLapInstall)?
            .0
        {
            CommandOutcome::FormationLapInstallPrepared { decision } => Ok(decision),
            _ => Err(Self::unexpected_outcome(
                "Formation Lap could not prepare the signed update.",
            )),
        }
    }

    fn cancel_formation_lap_install(&self, expected_version: String) -> Result<(), CommandError> {
        match self
            .execute_command(AppCommand::CancelFormationLapInstall { expected_version })?
            .0
        {
            CommandOutcome::FormationLapInstallCancelled => Ok(()),
            _ => Err(Self::unexpected_outcome(
                "Formation Lap could not release the update installation lease.",
            )),
        }
    }

    pub fn accept_recovery(&self) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::AcceptRecovery)
            .map(|(_, snapshot)| snapshot)
    }

    pub fn dismiss_recovery(&self) -> Result<AppSnapshot, CommandError> {
        self.execute_command(AppCommand::DismissRecovery)
            .map(|(_, snapshot)| snapshot)
    }

    pub fn discover_applications(&self) -> Result<DiscoverySnapshot, CommandError> {
        match self.execute_command(AppCommand::DiscoverApplications)?.0 {
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
        match self
            .execute_command(AppCommand::RecommendApplications {
                primary_sim_id: payload.primary_sim_id,
            })?
            .0
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

    fn unexpected_outcome(message: &str) -> CommandError {
        CommandError {
            code: "unexpected_outcome".to_owned(),
            message: message.to_owned(),
            recovery: Some("Try the action again.".to_owned()),
            diagnostic_id: None,
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
pub fn approve_profile(
    commands: tauri::State<'_, NativeCommandHost>,
    payload: ApproveProfilePayload,
) -> Result<AppSnapshot, CommandError> {
    commands.approve_profile(payload)
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
pub(crate) async fn start_session(
    commands: tauri::State<'_, NativeCommandHost>,
    coordinator: tauri::State<'_, UpdateCoordinator>,
    payload: ProfileIdPayload,
) -> Result<AppSnapshot, CommandError> {
    let coordinator = coordinator.inner().clone();
    let _session_start_barrier =
        tauri::async_runtime::spawn_blocking(move || coordinator.cancel_for_session_start())
            .await
            .map_err(|_| CommandError {
                code: "update_coordinator_failed".to_owned(),
                message: "Formation Lap could not safely join native update work.".to_owned(),
                recovery: Some(
                    "Wait for the update activity to finish, then try again.".to_owned(),
                ),
                diagnostic_id: None,
            })?
            .map_err(|message| CommandError {
                code: "activity_conflict".to_owned(),
                message,
                recovery: Some(
                    "Wait for the update activity to finish, then try again.".to_owned(),
                ),
                diagnostic_id: None,
            })?;
    commands.start_session(payload)
}

#[tauri::command]
pub fn test_game_launch(
    commands: tauri::State<'_, NativeCommandHost>,
    payload: ProfileIdPayload,
) -> Result<GameLaunchDiagnostic, CommandError> {
    commands.test_game_launch(payload)
}

#[tauri::command]
pub fn cancel_startup(
    commands: tauri::State<'_, NativeCommandHost>,
) -> Result<AppSnapshot, CommandError> {
    commands.cancel_startup()
}

#[tauri::command]
pub fn close_session(
    commands: tauri::State<'_, NativeCommandHost>,
) -> Result<AppSnapshot, CommandError> {
    commands.close_session()
}

#[tauri::command]
pub fn request_quit(
    app: tauri::AppHandle,
    commands: tauri::State<'_, NativeCommandHost>,
    payload: QuitPayload,
) -> Result<AppSnapshot, CommandError> {
    let (action, snapshot) = commands.request_quit(payload)?;
    match action {
        QuitAction::ExitNow => app.exit(0),
        QuitAction::WaitForSessionClose => {
            let app = app.clone();
            let commands = commands.inner().clone();
            thread::Builder::new()
                .name("formation-lap-quit".to_owned())
                .spawn(move || {
                    loop {
                        thread::sleep(std::time::Duration::from_millis(250));
                        let Ok(snapshot) = commands.refresh_processes() else {
                            break;
                        };
                        if snapshot.session.state == crate::SessionState::Idle {
                            app.exit(0);
                            break;
                        }
                    }
                })
                .map_err(|_| NativeCommandHost::worker_unavailable())?;
        }
    }
    Ok(snapshot)
}

#[tauri::command]
pub fn update_settings(
    commands: tauri::State<'_, NativeCommandHost>,
    payload: UpdateSettingsPayload,
) -> Result<AppSnapshot, CommandError> {
    let previous = commands.get_app_snapshot()?.settings.start_with_windows;
    crate::desktop_host::set_start_with_windows(payload.settings.start_with_windows).map_err(
        |_| CommandError {
            code: "startup_registration_failed".to_owned(),
            message: "Formation Lap could not update Start with Windows.".to_owned(),
            recovery: Some("Check current-user registry access and try again.".to_owned()),
            diagnostic_id: None,
        },
    )?;
    match commands.update_settings(payload) {
        Ok(snapshot) => Ok(snapshot),
        Err(error) => {
            let _ = crate::desktop_host::set_start_with_windows(previous);
            Err(error)
        }
    }
}

#[tauri::command]
pub fn export_diagnostics(
    commands: tauri::State<'_, NativeCommandHost>,
) -> Result<DiagnosticExport, CommandError> {
    commands.export_diagnostics()
}

pub(crate) async fn perform_update_check(
    commands: &NativeCommandHost,
    updater: &FormationLapUpdater,
    coordinator: &UpdateCoordinator,
    trigger: UpdateCheckTrigger,
) -> Result<AppSnapshot, CommandError> {
    let decision = commands.prepare_update_check(trigger)?;
    let UpdateCheckDecision::Planned(plan) = decision else {
        return commands.get_app_snapshot();
    };
    let update_lease = match coordinator.check(&plan.request_id) {
        Ok(lease) => lease,
        Err(message) => {
            let _ = commands.cancel_update_check(plan.request_id);
            return Err(CommandError {
                code: "activity_conflict".to_owned(),
                message,
                recovery: Some(
                    "Wait for the current Session or update activity to finish.".to_owned(),
                ),
                diagnostic_id: None,
            });
        }
    };
    let cancellation = update_lease.cancellation_token();
    let provider_plan = plan.clone();
    let provider_cancellation = cancellation.clone();
    let provider_results = tauri::async_runtime::spawn_blocking(move || {
        DirectUpdateProviderRuntime::new()
            .map(|runtime| {
                UpdateProviderRunner::new(runtime)
                    .check_with_cancellation(&provider_plan, &provider_cancellation)
            })
            .unwrap_or_else(|_| {
                provider_plan
                    .applications
                    .iter()
                    .map(|target| crate::ApplicationUpdateSnapshot {
                        application_id: target.application_id.clone(),
                        name: target.name.clone(),
                        status: UpdateStatus::Unknown {
                            reason: "The direct update providers could not start.".to_owned(),
                        },
                        information_url: None,
                    })
                    .collect()
            })
    });
    let formation_lap = updater
        .check(plan.channel, cancellation.clone())
        .await
        .unwrap_or_else(|reason| UpdateStatus::Unknown { reason });
    let applications = match provider_results.await {
        Ok(applications) => applications,
        Err(_) => {
            let _ = commands.cancel_update_check(plan.request_id);
            return Err(CommandError {
                code: "update_provider_failed".to_owned(),
                message: "Formation Lap could not finish the direct provider checks.".to_owned(),
                recovery: Some("Try the update check again later.".to_owned()),
                diagnostic_id: None,
            });
        }
    };
    if cancellation.is_cancelled() {
        return commands.cancel_update_check(plan.request_id);
    }
    commands.complete_update_check(UpdateCheckResult {
        request_id: plan.request_id,
        formation_lap,
        applications,
    })
}

#[tauri::command]
pub(crate) async fn check_updates(
    commands: tauri::State<'_, NativeCommandHost>,
    updater: tauri::State<'_, FormationLapUpdater>,
    coordinator: tauri::State<'_, UpdateCoordinator>,
) -> Result<AppSnapshot, CommandError> {
    perform_update_check(
        commands.inner(),
        updater.inner(),
        coordinator.inner(),
        UpdateCheckTrigger::Manual,
    )
    .await
}

#[tauri::command]
pub(crate) async fn install_formation_lap_update(
    app: tauri::AppHandle,
    commands: tauri::State<'_, NativeCommandHost>,
    updater: tauri::State<'_, FormationLapUpdater>,
    coordinator: tauri::State<'_, UpdateCoordinator>,
) -> Result<AppSnapshot, CommandError> {
    let checked_version = match commands.get_app_snapshot()?.updates.formation_lap {
        UpdateStatus::UpdateAvailable { latest_version, .. } => latest_version,
        UpdateStatus::Current { .. } | UpdateStatus::Unknown { .. } => {
            return Err(CommandError {
                code: "no_signed_update".to_owned(),
                message: "No checked Formation Lap update is ready to install.".to_owned(),
                recovery: Some("Run Check now first.".to_owned()),
                diagnostic_id: None,
            });
        }
    };
    let install_lease = coordinator
        .install(&checked_version)
        .map_err(|message| CommandError {
            code: "activity_conflict".to_owned(),
            message,
            recovery: Some("Wait for the current Session or update activity to finish.".to_owned()),
            diagnostic_id: None,
        })?;
    match commands.prepare_formation_lap_install()? {
        FormationLapInstallDecision::Ready { latest_version }
            if latest_version == checked_version =>
        {
            let channel = commands.get_app_snapshot()?.settings.update_channel;
            if let Err(message) = updater.install(&app, channel, &latest_version).await {
                let _ = commands.cancel_formation_lap_install(latest_version);
                return Err(CommandError {
                    code: "signed_update_rejected".to_owned(),
                    message,
                    recovery: Some(
                        "Run a fresh update check or install an official signed release."
                            .to_owned(),
                    ),
                    diagnostic_id: None,
                });
            }
            std::mem::forget(install_lease);
            commands.get_app_snapshot()
        }
        FormationLapInstallDecision::Ready { latest_version } => {
            let _ = commands.cancel_formation_lap_install(latest_version);
            Err(CommandError {
                code: "signed_update_rejected".to_owned(),
                message: "The selected update changed; run a fresh signed update check.".to_owned(),
                recovery: Some("Run Check now again before installing.".to_owned()),
                diagnostic_id: None,
            })
        }
        FormationLapInstallDecision::Deferred => Err(CommandError {
            code: "race_safe_update_deferred".to_owned(),
            message: "The signed update will wait until the Session is idle.".to_owned(),
            recovery: Some("Close the Session, then install the update.".to_owned()),
            diagnostic_id: None,
        }),
        FormationLapInstallDecision::NoUpdate => Err(CommandError {
            code: "no_signed_update".to_owned(),
            message: "No checked Formation Lap update is ready to install.".to_owned(),
            recovery: Some("Run Check now first.".to_owned()),
            diagnostic_id: None,
        }),
    }
}

#[tauri::command]
pub fn accept_recovery(
    commands: tauri::State<'_, NativeCommandHost>,
) -> Result<AppSnapshot, CommandError> {
    commands.accept_recovery()
}

#[tauri::command]
pub fn dismiss_recovery(
    commands: tauri::State<'_, NativeCommandHost>,
) -> Result<AppSnapshot, CommandError> {
    commands.dismiss_recovery()
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
