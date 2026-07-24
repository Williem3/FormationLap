use crate::{
    AppCommand, AppSnapshot, CommandOutcome, CoreError, DesktopSettings, DiagnosticExport,
    DiscoverySnapshot, FormationLapCore, GameLaunchDiagnostic, QuitAction, QuitDisposition,
    RacingProfile, SupportingApplicationRecommendation, TargetedDiscoverySources,
    WindowCloseAction,
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
pub fn start_session(
    commands: tauri::State<'_, NativeCommandHost>,
    payload: ProfileIdPayload,
) -> Result<AppSnapshot, CommandError> {
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
