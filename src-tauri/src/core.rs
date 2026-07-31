use crate::diagnostics::DiagnosticLog;
use crate::discovery_catalog::{
    DiscoveryCatalog, DiscoveryCatalogError, TargetedDiscoverySources, executable_icon,
};
use crate::game_launch_diagnostics::GameLaunchDiagnostics;
use crate::update_advisor::UpdateAdvisor;
use crate::{
    AppSnapshot, ApplicationIcon, ApplicationIconSnapshot, ApplicationProcessSnapshot,
    DevelopmentPrivilegeBroker, ElevatedOperation, ElevatedOperationResult, NewRacingProfile,
    PrivilegeBroker, PrivilegeBrokerError, ProcessIdentity, ProcessObservation, ProcessOwnership,
    ProcessResponsiveness, ProcessRuntime, ProcessRuntimeError, ProcessStatus, ProfileLibrary,
    RacingProfile, SettingsStore, WindowsPrivilegeBroker, WindowsProcessRuntime,
    session_journal::SessionJournal,
};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt, io,
    time::{Duration, Instant},
};

/// User intent accepted by FormationLapCore.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppCommand {
    CreateProfile {
        profile: Box<NewRacingProfile>,
    },
    EditProfile {
        profile_id: String,
        name: String,
        primary_sim_name: String,
    },
    DeleteProfile {
        profile_id: String,
    },
    DuplicateProfile {
        source_profile_id: String,
        name: String,
    },
    SaveProfile {
        profile: Box<RacingProfile>,
    },
    SelectProfile {
        profile_id: String,
    },
    ExportProfile {
        profile_id: String,
    },
    ImportProfile {
        document: String,
    },
    ApproveProfile {
        profile_id: String,
        configuration_reviewed: bool,
        approved_privileged_application_ids: Vec<String>,
    },
    StartApplication {
        profile_id: String,
        application_id: String,
    },
    ExitApplication {
        application_id: String,
        pre_existing_confirmed: bool,
    },
    ConfirmProcessAction {
        token: String,
    },
    CancelProcessAction {
        token: String,
    },
    RestartApplication {
        profile_id: String,
        application_id: String,
        pre_existing_confirmed: bool,
    },
    StartSession {
        profile_id: String,
    },
    CancelStartup,
    CloseSession,
    AcceptRecovery,
    DismissRecovery,
    DiscoverApplications,
    RecommendApplications {
        primary_sim_id: String,
    },
    TestGameLaunch {
        profile_id: String,
    },
    RequestWindowClose,
    RequestQuit {
        disposition: crate::QuitDisposition,
    },
    UpdateSettings {
        settings: crate::DesktopSettings,
    },
    ExportDiagnostics,
    PrepareUpdateCheck {
        trigger: crate::UpdateCheckTrigger,
        now_unix_seconds: u64,
    },
    CompleteUpdateCheck {
        result: crate::UpdateCheckResult,
    },
    CancelUpdateCheck {
        request_id: String,
    },
    PrepareFormationLapInstall,
    CancelFormationLapInstall {
        expected_version: String,
    },
    RefreshProcesses,
}

impl AppCommand {
    fn diagnostic_label(&self) -> &'static str {
        match self {
            Self::CreateProfile { .. } => "profile.create",
            Self::EditProfile { .. } => "profile.edit",
            Self::DeleteProfile { .. } => "profile.delete",
            Self::DuplicateProfile { .. } => "profile.duplicate",
            Self::SaveProfile { .. } => "profile.save",
            Self::SelectProfile { .. } => "profile.select",
            Self::ExportProfile { .. } => "profile.export",
            Self::ImportProfile { .. } => "profile.import",
            Self::ApproveProfile { .. } => "profile.approve",
            Self::StartApplication { .. } => "application.start",
            Self::ExitApplication { .. } => "application.exit",
            Self::ConfirmProcessAction { .. } => "application.confirm",
            Self::CancelProcessAction { .. } => "application.cancel",
            Self::RestartApplication { .. } => "application.restart",
            Self::StartSession { .. } => "session.start",
            Self::CancelStartup => "session.cancel_startup",
            Self::CloseSession => "session.close",
            Self::AcceptRecovery => "recovery.accept",
            Self::DismissRecovery => "recovery.dismiss",
            Self::DiscoverApplications => "discovery.scan",
            Self::RecommendApplications { .. } => "discovery.recommend",
            Self::TestGameLaunch { .. } => "game.test_launch",
            Self::RequestWindowClose => "desktop.window_close",
            Self::RequestQuit { .. } => "desktop.quit",
            Self::UpdateSettings { .. } => "settings.update",
            Self::ExportDiagnostics => "diagnostics.export",
            Self::PrepareUpdateCheck { .. } => "updates.prepare_check",
            Self::CompleteUpdateCheck { .. } => "updates.complete_check",
            Self::CancelUpdateCheck { .. } => "updates.cancel_check",
            Self::PrepareFormationLapInstall => "updates.prepare_install",
            Self::CancelFormationLapInstall { .. } => "updates.cancel_install",
            Self::RefreshProcesses => "process.refresh",
        }
    }
}

/// Observable result of a completed FormationLapCore command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandOutcome {
    ProfileCreated {
        profile_id: String,
    },
    ProfileUpdated {
        profile_id: String,
    },
    ProfileDeleted {
        profile_id: String,
    },
    ProfileSelected {
        profile_id: String,
    },
    ProfileExported {
        document: String,
    },
    ApplicationStartRequested {
        application_id: String,
    },
    ApplicationAlreadyRunning {
        application_id: String,
    },
    ApplicationStopped {
        application_id: String,
    },
    ApplicationRestarted {
        application_id: String,
    },
    SessionStartRequested {
        profile_id: String,
    },
    SessionStartFailed {
        application_id: String,
    },
    SessionCancellationRequested,
    SessionCloseRequested,
    RecoveryAccepted,
    RecoveryDismissed,
    PreExistingControlConfirmationRequired {
        application_id: String,
    },
    ForceStopConfirmationRequired {
        application_id: String,
    },
    ApplicationsDiscovered {
        discovery: crate::DiscoverySnapshot,
    },
    ApplicationsRecommended {
        recommendations: Vec<crate::SupportingApplicationRecommendation>,
    },
    GameLaunchTested {
        diagnostic: crate::GameLaunchDiagnostic,
    },
    WindowCloseRequested {
        action: crate::WindowCloseAction,
    },
    QuitRequested {
        action: crate::QuitAction,
    },
    SettingsUpdated,
    DiagnosticsExported {
        diagnostics: crate::DiagnosticExport,
    },
    UpdateCheckPrepared {
        decision: crate::UpdateCheckDecision,
    },
    UpdateCheckCompleted,
    UpdateCheckCancelled,
    FormationLapInstallPrepared {
        decision: crate::FormationLapInstallDecision,
    },
    FormationLapInstallCancelled,
    ProcessesRefreshed,
}

#[derive(Debug)]
pub enum CoreError {
    Storage(io::Error),
    InvalidProfileDocument(serde_json::Error),
    InvalidSettingsDocument(serde_json::Error),
    InvalidSessionJournal(serde_json::Error),
    InvalidGameLaunchDiagnostic(serde_json::Error),
    InvalidLaunchRecipe(String),
    InvalidProfileName(&'static str),
    ProfileNotFound(String),
    ProfileNeedsReview(String),
    InvalidProfileApproval(String),
    InvalidProcessConfirmation,
    ApplicationNotFound(String),
    ProcessRuntime(ProcessRuntimeError),
    PrivilegeBroker(PrivilegeBrokerError),
    DiscoveryCatalog(DiscoveryCatalogError),
    InvalidUpdateCheck(String),
    ActivityConflict {
        activity: &'static str,
        command: &'static str,
    },
    InvalidSessionTransition {
        current: crate::SessionState,
        command: &'static str,
    },
    UnsupportedProfileSchema(u32),
    UnsupportedSettingsSchema(u32),
    UnsupportedSessionJournalSchema(u32),
}

impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => write!(formatter, "profile storage failed: {error}"),
            Self::InvalidProfileDocument(error) => {
                write!(formatter, "profile document is invalid: {error}")
            }
            Self::InvalidSettingsDocument(error) => {
                write!(formatter, "settings document is invalid: {error}")
            }
            Self::InvalidSessionJournal(error) => {
                write!(formatter, "active Session journal is invalid: {error}")
            }
            Self::InvalidGameLaunchDiagnostic(error) => {
                write!(formatter, "Test Game Launch diagnostic is invalid: {error}")
            }
            Self::InvalidLaunchRecipe(message) => {
                write!(formatter, "Launch Recipe is invalid: {message}")
            }
            Self::InvalidProfileName(field) => {
                write!(formatter, "{field} must not be blank")
            }
            Self::ProfileNotFound(profile_id) => {
                write!(formatter, "Racing Profile {profile_id} was not found")
            }
            Self::ProfileNeedsReview(profile_id) => {
                write!(
                    formatter,
                    "Racing Profile {profile_id} must be reviewed before starting a Session"
                )
            }
            Self::InvalidProfileApproval(message) => {
                write!(formatter, "Racing Profile approval is invalid: {message}")
            }
            Self::InvalidProcessConfirmation => {
                write!(formatter, "process confirmation is no longer valid")
            }
            Self::ApplicationNotFound(application_id) => {
                write!(formatter, "application {application_id} was not found")
            }
            Self::ProcessRuntime(error) => write!(formatter, "process runtime failed: {error}"),
            Self::PrivilegeBroker(error) => {
                write!(formatter, "privileged operation failed: {error}")
            }
            Self::DiscoveryCatalog(error) => write!(formatter, "catalog discovery failed: {error}"),
            Self::InvalidUpdateCheck(message) => {
                write!(formatter, "update check is invalid: {message}")
            }
            Self::ActivityConflict { activity, command } => {
                write!(formatter, "{command} cannot overlap {activity}")
            }
            Self::InvalidSessionTransition { current, command } => {
                write!(
                    formatter,
                    "{command} is not available while the Session is {current:?}"
                )
            }
            Self::UnsupportedProfileSchema(version) => {
                write!(
                    formatter,
                    "profile schema version {version} is not supported"
                )
            }
            Self::UnsupportedSettingsSchema(version) => {
                write!(
                    formatter,
                    "settings schema version {version} is not supported"
                )
            }
            Self::UnsupportedSessionJournalSchema(version) => {
                write!(
                    formatter,
                    "active Session journal schema version {version} is not supported"
                )
            }
        }
    }
}

impl Error for CoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            Self::InvalidProfileDocument(error) => Some(error),
            Self::InvalidSettingsDocument(error) => Some(error),
            Self::InvalidSessionJournal(error) => Some(error),
            Self::InvalidGameLaunchDiagnostic(error) => Some(error),
            Self::InvalidProfileName(_)
            | Self::InvalidLaunchRecipe(_)
            | Self::InvalidUpdateCheck(_)
            | Self::ActivityConflict { .. }
            | Self::ProfileNotFound(_)
            | Self::ProfileNeedsReview(_)
            | Self::InvalidProfileApproval(_)
            | Self::InvalidProcessConfirmation
            | Self::ApplicationNotFound(_)
            | Self::InvalidSessionTransition { .. }
            | Self::UnsupportedProfileSchema(_)
            | Self::UnsupportedSettingsSchema(_)
            | Self::UnsupportedSessionJournalSchema(_) => None,
            Self::ProcessRuntime(error) => Some(error),
            Self::PrivilegeBroker(error) => Some(error),
            Self::DiscoveryCatalog(error) => Some(error),
        }
    }
}

impl From<io::Error> for CoreError {
    fn from(error: io::Error) -> Self {
        Self::Storage(error)
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidProfileDocument(error)
    }
}

impl From<ProcessRuntimeError> for CoreError {
    fn from(error: ProcessRuntimeError) -> Self {
        Self::ProcessRuntime(error)
    }
}

impl From<PrivilegeBrokerError> for CoreError {
    fn from(error: PrivilegeBrokerError) -> Self {
        Self::PrivilegeBroker(error)
    }
}

impl From<DiscoveryCatalogError> for CoreError {
    fn from(error: DiscoveryCatalogError) -> Self {
        Self::DiscoveryCatalog(error)
    }
}

/// Owns authoritative Racing Profile and Session state.
pub struct FormationLapCore {
    profile_library: ProfileLibrary,
    settings_store: SettingsStore,
    diagnostic_log: DiagnosticLog,
    process_runtime: Box<dyn ProcessRuntime>,
    privilege_broker: Box<dyn PrivilegeBroker>,
    application_processes: BTreeMap<String, ApplicationProcessSnapshot>,
    failed_responsiveness_checks: BTreeMap<String, u8>,
    application_recipes: BTreeMap<String, crate::LaunchRecipe>,
    pending_process_confirmation: Option<PendingProcessConfirmation>,
    prepared_elevated_launches: BTreeMap<String, PreparedElevatedLaunch>,
    startup_started_at: BTreeMap<String, Instant>,
    post_start_ready_at: BTreeMap<String, Instant>,
    session_events: Vec<crate::SessionEvent>,
    session_journal: SessionJournal,
    discovery_catalog: DiscoveryCatalog,
    game_launch_diagnostics: GameLaunchDiagnostics,
    update_advisor: UpdateAdvisor,
    formation_lap_installing: Option<String>,
    session: crate::SessionSnapshot,
}

#[derive(Clone)]
enum PreparedElevatedLaunch {
    SessionOwned(ProcessIdentity),
    PreExisting(ProcessIdentity),
    Failed(String),
}

#[derive(Clone)]
struct PendingProcessConfirmation {
    snapshot: crate::ProcessConfirmationSnapshot,
    restart_recipe: Option<crate::LaunchRecipe>,
}

impl FormationLapCore {
    pub fn open(storage_root: impl AsRef<std::path::Path>) -> Result<Self, CoreError> {
        Self::open_with_runtime_and_discovery_sources(
            storage_root,
            WindowsProcessRuntime::new(),
            WindowsPrivilegeBroker::new()?,
            TargetedDiscoverySources::windows_defaults(),
        )
    }

    pub fn open_with_discovery_sources(
        storage_root: impl AsRef<std::path::Path>,
        sources: TargetedDiscoverySources,
    ) -> Result<Self, CoreError> {
        Self::open_with_runtime_and_discovery_sources(
            storage_root,
            WindowsProcessRuntime::new(),
            WindowsPrivilegeBroker::new()?,
            sources,
        )
    }

    pub fn open_with_runtime(
        storage_root: impl AsRef<std::path::Path>,
        process_runtime: impl ProcessRuntime + 'static,
    ) -> Result<Self, CoreError> {
        Self::open_with_runtime_and_discovery_sources(
            storage_root,
            process_runtime,
            DevelopmentPrivilegeBroker::default(),
            TargetedDiscoverySources::default(),
        )
    }

    pub fn open_with_runtime_and_privilege_broker(
        storage_root: impl AsRef<std::path::Path>,
        process_runtime: impl ProcessRuntime + 'static,
        privilege_broker: impl PrivilegeBroker + 'static,
    ) -> Result<Self, CoreError> {
        Self::open_with_runtime_and_discovery_sources(
            storage_root,
            process_runtime,
            privilege_broker,
            TargetedDiscoverySources::default(),
        )
    }

    fn open_with_runtime_and_discovery_sources(
        storage_root: impl AsRef<std::path::Path>,
        mut process_runtime: impl ProcessRuntime + 'static,
        privilege_broker: impl PrivilegeBroker + 'static,
        discovery_sources: TargetedDiscoverySources,
    ) -> Result<Self, CoreError> {
        let storage_root = storage_root.as_ref();
        let profile_library = ProfileLibrary::open(storage_root)?;
        let session_journal = SessionJournal::open(storage_root)?;
        let mut session = crate::SessionSnapshot::default();
        let mut application_processes = BTreeMap::new();
        let mut application_recipes = BTreeMap::new();
        if let Some(recovered) = session_journal.load()? {
            let profile = recovered
                .session
                .active_profile_id
                .as_deref()
                .and_then(|profile_id| profile_library.profile(profile_id));
            let mut recovered_recipes = BTreeMap::new();
            if let Some(profile) = &profile {
                for application in profile
                    .supporting_applications
                    .iter()
                    .map(|supporting| &supporting.application)
                    .chain(std::iter::once(&profile.primary_sim))
                {
                    recovered_recipes
                        .insert(application.id.clone(), application.launch_recipe.clone());
                }
            }
            let journal_is_consistent =
                profile.is_some()
                    && recovered.application_processes.iter().all(|process| {
                        let Some(identity) = process.identity.as_ref() else {
                            return true;
                        };
                        recovered
                            .session
                            .applications
                            .iter()
                            .any(|application| application.application_id == process.application_id)
                            && recovered_recipes.get(&process.application_id).is_some_and(
                                |recipe| recovery_identity_matches_recipe(identity, recipe),
                            )
                    });
            if journal_is_consistent {
                for mut process in recovered.application_processes {
                    let Some(identity) = process.identity.as_ref() else {
                        continue;
                    };
                    if matches!(
                        process_runtime.observe(identity),
                        Ok(ProcessObservation::Running { .. })
                    ) {
                        // A journal is user-writable. Liveness and recipe matching
                        // make a Recovery Offer safe to observe, but cannot prove
                        // that Formation Lap created the Process. Recovery therefore
                        // never restores automatic-cleanup ownership.
                        process.ownership = Some(ProcessOwnership::PreExisting);
                        process.status = ProcessStatus::RunningPreExisting;
                        application_processes.insert(process.application_id.clone(), process);
                    }
                }
            }
            if !application_processes.is_empty() {
                session = recovered.session;
                session.state = crate::SessionState::RecoveryAvailable;
                session.summary = None;
                for application in &mut session.applications {
                    if let Some(process) = application_processes.get(&application.application_id) {
                        application.state =
                            if process.ownership == Some(ProcessOwnership::PreExisting) {
                                crate::SessionApplicationState::RunningPreExisting
                            } else {
                                crate::SessionApplicationState::Running
                            };
                    }
                }
                application_recipes = recovered_recipes;
            } else {
                session_journal.clear()?;
            }
        }
        let settings_store = SettingsStore::open(storage_root)?;
        let update_advisor =
            UpdateAdvisor::new(settings_store.last_automatic_update_check_unix_seconds());
        Ok(Self {
            profile_library,
            settings_store,
            diagnostic_log: DiagnosticLog::open(storage_root)?,
            process_runtime: Box::new(process_runtime),
            privilege_broker: Box::new(privilege_broker),
            application_processes,
            failed_responsiveness_checks: BTreeMap::new(),
            application_recipes,
            pending_process_confirmation: None,
            prepared_elevated_launches: BTreeMap::new(),
            startup_started_at: BTreeMap::new(),
            post_start_ready_at: BTreeMap::new(),
            session_events: Vec::new(),
            session_journal,
            discovery_catalog: DiscoveryCatalog::bundled_with_sources(discovery_sources)?,
            game_launch_diagnostics: GameLaunchDiagnostics::open(storage_root)?,
            update_advisor,
            formation_lap_installing: None,
            session,
        })
    }

    pub fn snapshot(&self) -> AppSnapshot {
        let mut snapshot = AppSnapshot::foundation();
        snapshot.profiles = self.profile_library.summaries();
        snapshot.selected_profile = self
            .settings_store
            .selected_profile_id()
            .and_then(|profile_id| self.profile_library.profile(profile_id))
            .or_else(|| self.profile_library.selected_profile());
        let selected_profile_id = snapshot
            .selected_profile
            .as_ref()
            .map(|profile| profile.id.as_str());
        snapshot.application_icons = Some(
            self.profile_library
                .profiles()
                .flat_map(|profile| {
                    self.profile_application_icons(
                        &profile,
                        selected_profile_id == Some(profile.id.as_str()),
                    )
                })
                .collect(),
        );
        snapshot.settings = self.settings_store.desktop().clone();
        snapshot.updates = self.update_advisor.snapshot(
            self.settings_store
                .last_automatic_update_check_unix_seconds(),
        );
        snapshot.application_processes = self.application_processes.values().cloned().collect();
        snapshot.pending_process_confirmation = self
            .pending_process_confirmation
            .as_ref()
            .map(|pending| pending.snapshot.clone());
        snapshot.session = self.session.clone();
        snapshot
    }

    fn request_process_confirmation(
        &mut self,
        application_id: String,
        action: crate::ProcessConfirmationAction,
        identity: ProcessIdentity,
        restart_recipe: Option<crate::LaunchRecipe>,
    ) {
        self.pending_process_confirmation = Some(PendingProcessConfirmation {
            snapshot: crate::ProcessConfirmationSnapshot {
                token: uuid::Uuid::new_v4().to_string(),
                application_id,
                action,
                identity,
            },
            restart_recipe,
        });
    }

    fn confirm_process_action(&mut self, token: String) -> Result<CommandOutcome, CoreError> {
        self.ensure_force_stop_is_available()?;
        if !self
            .pending_process_confirmation
            .as_ref()
            .is_some_and(|pending| pending.snapshot.token == token)
        {
            return Err(CoreError::InvalidProcessConfirmation);
        }
        let pending = self
            .pending_process_confirmation
            .take()
            .expect("matching pending Process confirmation should remain present");
        let application_id = pending.snapshot.application_id.clone();
        let process = self
            .application_processes
            .get(&application_id)
            .cloned()
            .ok_or(CoreError::InvalidProcessConfirmation)?;
        if process.identity.as_ref() != Some(&pending.snapshot.identity) {
            return Err(CoreError::InvalidProcessConfirmation);
        }
        let elevated = self
            .application_recipes
            .get(&application_id)
            .is_some_and(|recipe| recipe.elevated);
        self.force_stop_process(&pending.snapshot.identity, elevated)?;
        let process = self
            .application_processes
            .get_mut(&application_id)
            .expect("confirmed application Process should remain present");
        process.status = ProcessStatus::Stopped;
        process.ownership = None;
        process.identity = None;
        self.failed_responsiveness_checks.remove(&application_id);
        self.sync_session_application_states();
        if self.session.state == crate::SessionState::Closing {
            self.advance_session_close()?;
        }
        if pending.snapshot.action == crate::ProcessConfirmationAction::Restart {
            let recipe = pending
                .restart_recipe
                .ok_or(CoreError::InvalidProcessConfirmation)?;
            let ownership = self.launch_or_adopt(&application_id, recipe)?;
            return Ok(if ownership == ProcessOwnership::PreExisting {
                CommandOutcome::ApplicationAlreadyRunning { application_id }
            } else {
                CommandOutcome::ApplicationRestarted { application_id }
            });
        }
        Ok(CommandOutcome::ApplicationStopped { application_id })
    }

    fn profile_application_icons(
        &self,
        profile: &RacingProfile,
        include_supporting_applications: bool,
    ) -> Vec<ApplicationIconSnapshot> {
        profile
            .supporting_applications
            .iter()
            .filter(move |_| include_supporting_applications)
            .map(|supporting| &supporting.application)
            .chain(std::iter::once(&profile.primary_sim))
            .map(|application| {
                let icon = match &application.launch_recipe.source {
                    crate::LaunchSource::DirectExecutable { executable_path } => {
                        let icon = executable_icon(std::path::Path::new(executable_path));
                        if matches!(icon, ApplicationIcon::Generic)
                            && application.id == profile.primary_sim.id
                        {
                            self.discovery_catalog
                                .installed_primary_sim_icon(&application.name)
                                .unwrap_or(icon)
                        } else {
                            icon
                        }
                    }
                    crate::LaunchSource::Steam { app_id, .. } => application
                        .launch_recipe
                        .monitored_executable_path
                        .as_ref()
                        .map(|path| executable_icon(std::path::Path::new(path)))
                        .filter(|icon| matches!(icon, ApplicationIcon::LocalData { .. }))
                        .unwrap_or_else(|| {
                            self.discovery_catalog.steam_library_cache_icon(*app_id)
                        }),
                };
                ApplicationIconSnapshot {
                    application_id: application.id.clone(),
                    icon,
                }
            })
            .collect()
    }

    pub fn execute(&mut self, command: AppCommand) -> Result<CommandOutcome, CoreError> {
        let diagnostic_label = command.diagnostic_label();
        let record_success = !matches!(
            &command,
            AppCommand::RefreshProcesses
                | AppCommand::PrepareUpdateCheck {
                    trigger: crate::UpdateCheckTrigger::Automatic,
                    ..
                }
        );
        match self.execute_inner(command) {
            Ok(outcome) => {
                self.update_advisor
                    .release_deferred_if_safe(&self.session.state);
                if let Err(error) = self.sync_session_journal() {
                    let _ = self.diagnostic_log.record(diagnostic_label, "failed");
                    return Err(error);
                }
                if record_success {
                    let _ = self.diagnostic_log.record(diagnostic_label, "succeeded");
                }
                Ok(outcome)
            }
            Err(error) => {
                let _ = self.diagnostic_log.record(diagnostic_label, "failed");
                Err(error)
            }
        }
    }

    fn execute_inner(&mut self, command: AppCommand) -> Result<CommandOutcome, CoreError> {
        match command {
            AppCommand::CreateProfile { profile } => {
                let profile_id = self.profile_library.create_complete(*profile)?;
                if let Err(error) = self.settings_store.select_profile(profile_id.clone()) {
                    self.profile_library.discard_created(&profile_id)?;
                    return Err(error);
                }
                Ok(CommandOutcome::ProfileCreated { profile_id })
            }
            AppCommand::EditProfile {
                profile_id,
                name,
                primary_sim_name,
            } => {
                self.ensure_active_profile_is_editable(&profile_id)?;
                self.profile_library
                    .edit(&profile_id, name, primary_sim_name)?;
                Ok(CommandOutcome::ProfileUpdated { profile_id })
            }
            AppCommand::DeleteProfile { profile_id } => {
                self.ensure_active_profile_is_editable(&profile_id)?;
                self.profile_library.delete(&profile_id)?;
                Ok(CommandOutcome::ProfileDeleted { profile_id })
            }
            AppCommand::DuplicateProfile {
                source_profile_id,
                name,
            } => {
                let profile_id = self.profile_library.duplicate(&source_profile_id, name)?;
                Ok(CommandOutcome::ProfileCreated { profile_id })
            }
            AppCommand::SaveProfile { profile } => {
                let profile_id = profile.id.clone();
                self.ensure_active_profile_is_editable(&profile_id)?;
                self.profile_library.save(*profile)?;
                Ok(CommandOutcome::ProfileUpdated { profile_id })
            }
            AppCommand::SelectProfile { profile_id } => {
                if !self.profile_library.contains(&profile_id) {
                    return Err(CoreError::ProfileNotFound(profile_id));
                }
                self.settings_store.select_profile(profile_id.clone())?;
                Ok(CommandOutcome::ProfileSelected { profile_id })
            }
            AppCommand::ExportProfile { profile_id } => {
                let document = self.profile_library.export(&profile_id)?;
                Ok(CommandOutcome::ProfileExported { document })
            }
            AppCommand::ImportProfile { document } => {
                let profile_id = self.profile_library.import(&document)?;
                Ok(CommandOutcome::ProfileCreated { profile_id })
            }
            AppCommand::ApproveProfile {
                profile_id,
                configuration_reviewed,
                approved_privileged_application_ids,
            } => {
                self.ensure_active_profile_is_editable(&profile_id)?;
                self.profile_library.approve(
                    &profile_id,
                    configuration_reviewed,
                    &approved_privileged_application_ids,
                )?;
                Ok(CommandOutcome::ProfileUpdated { profile_id })
            }
            AppCommand::StartApplication {
                profile_id,
                application_id,
            } => {
                self.ensure_manual_lifecycle_is_available("Start Application")?;
                let profile = self
                    .profile_library
                    .profile(&profile_id)
                    .ok_or_else(|| CoreError::ProfileNotFound(profile_id.clone()))?;
                let application = if profile.primary_sim.id == application_id {
                    Some(self.resolved_primary_sim(&profile)?)
                } else {
                    profile
                        .supporting_applications
                        .iter()
                        .map(|supporting| supporting.application.clone())
                        .find(|application| application.id == application_id)
                }
                .ok_or_else(|| CoreError::ApplicationNotFound(application_id.clone()))?;
                let launch_recipe = application.launch_recipe.clone();
                let ownership = self.launch_or_adopt(&application_id, launch_recipe)?;
                Ok(if ownership == ProcessOwnership::PreExisting {
                    CommandOutcome::ApplicationAlreadyRunning { application_id }
                } else {
                    CommandOutcome::ApplicationStartRequested { application_id }
                })
            }
            AppCommand::ExitApplication {
                application_id,
                pre_existing_confirmed,
            } => {
                self.ensure_manual_lifecycle_is_available("Exit Application")?;
                let process = self
                    .application_processes
                    .get(&application_id)
                    .cloned()
                    .ok_or_else(|| CoreError::ApplicationNotFound(application_id.clone()))?;
                if process.ownership == Some(ProcessOwnership::PreExisting)
                    && !pre_existing_confirmed
                {
                    return Ok(CommandOutcome::PreExistingControlConfirmationRequired {
                        application_id,
                    });
                }
                let Some(identity) = process.identity else {
                    return Ok(CommandOutcome::ApplicationStopped { application_id });
                };
                let strategy = self
                    .application_recipes
                    .get(&application_id)
                    .map(|recipe| (recipe.shutdown_strategy.clone(), recipe.elevated))
                    .ok_or_else(|| CoreError::ApplicationNotFound(application_id.clone()))?;
                let (graceful, exited) = self.request_graceful_stop(
                    &application_id,
                    &identity,
                    &strategy.0,
                    strategy.1,
                )?;
                self.application_processes
                    .get_mut(&application_id)
                    .expect("application Process should remain present")
                    .status = ProcessStatus::Stopping;

                if graceful == crate::GracefulStopResult::Requested && exited {
                    let process = self
                        .application_processes
                        .get_mut(&application_id)
                        .expect("application Process should remain present");
                    process.status = ProcessStatus::Stopped;
                    process.ownership = None;
                    process.identity = None;
                    self.failed_responsiveness_checks.remove(&application_id);
                    Ok(CommandOutcome::ApplicationStopped { application_id })
                } else {
                    self.request_process_confirmation(
                        application_id.clone(),
                        crate::ProcessConfirmationAction::Exit,
                        identity,
                        None,
                    );
                    Ok(CommandOutcome::ForceStopConfirmationRequired { application_id })
                }
            }
            AppCommand::ConfirmProcessAction { token } => self.confirm_process_action(token),
            AppCommand::CancelProcessAction { token } => {
                if self
                    .pending_process_confirmation
                    .as_ref()
                    .is_some_and(|pending| pending.snapshot.token == token)
                {
                    self.pending_process_confirmation = None;
                }
                Ok(CommandOutcome::ProcessesRefreshed)
            }
            AppCommand::RestartApplication {
                profile_id,
                application_id,
                pre_existing_confirmed,
            } => {
                self.ensure_manual_lifecycle_is_available("Restart Application")?;
                let profile = self
                    .profile_library
                    .profile(&profile_id)
                    .ok_or_else(|| CoreError::ProfileNotFound(profile_id.clone()))?;
                let application = if profile.primary_sim.id == application_id {
                    Some(self.resolved_primary_sim(&profile)?)
                } else {
                    profile
                        .supporting_applications
                        .iter()
                        .map(|supporting| supporting.application.clone())
                        .find(|application| application.id == application_id)
                }
                .ok_or_else(|| CoreError::ApplicationNotFound(application_id.clone()))?;
                let recipe = application.launch_recipe.clone();
                let process = self
                    .application_processes
                    .get(&application_id)
                    .cloned()
                    .ok_or_else(|| CoreError::ApplicationNotFound(application_id.clone()))?;
                if process.ownership == Some(ProcessOwnership::PreExisting)
                    && !pre_existing_confirmed
                {
                    return Ok(CommandOutcome::PreExistingControlConfirmationRequired {
                        application_id,
                    });
                }
                let Some(identity) = process.identity else {
                    let ownership = self.launch_or_adopt(&application_id, recipe)?;
                    return Ok(if ownership == ProcessOwnership::PreExisting {
                        CommandOutcome::ApplicationAlreadyRunning { application_id }
                    } else {
                        CommandOutcome::ApplicationRestarted { application_id }
                    });
                };
                let (graceful, exited) = self.request_graceful_stop(
                    &application_id,
                    &identity,
                    &recipe.shutdown_strategy,
                    recipe.elevated,
                )?;
                self.application_processes
                    .get_mut(&application_id)
                    .expect("application Process should remain present")
                    .status = ProcessStatus::Stopping;

                if graceful == crate::GracefulStopResult::Requested && exited {
                    let ownership = self.launch_or_adopt(&application_id, recipe)?;
                    Ok(if ownership == ProcessOwnership::PreExisting {
                        CommandOutcome::ApplicationAlreadyRunning { application_id }
                    } else {
                        CommandOutcome::ApplicationRestarted { application_id }
                    })
                } else {
                    self.request_process_confirmation(
                        application_id.clone(),
                        crate::ProcessConfirmationAction::Restart,
                        identity,
                        Some(recipe),
                    );
                    Ok(CommandOutcome::ForceStopConfirmationRequired { application_id })
                }
            }
            AppCommand::StartSession { profile_id } => {
                self.pending_process_confirmation = None;
                if self.session.state != crate::SessionState::Idle {
                    return Err(CoreError::InvalidSessionTransition {
                        current: self.session.state.clone(),
                        command: "Start Session",
                    });
                }
                if self.formation_lap_installing.is_some() {
                    return Err(CoreError::ActivityConflict {
                        activity: "Formation Lap update installation",
                        command: "Start Session",
                    });
                }
                if self.profile_library.requires_review(&profile_id)? {
                    return Err(CoreError::ProfileNeedsReview(profile_id));
                }
                let profile = self
                    .profile_library
                    .profile(&profile_id)
                    .ok_or_else(|| CoreError::ProfileNotFound(profile_id.clone()))?;
                let mut applications = profile
                    .supporting_applications
                    .iter()
                    .map(|supporting| crate::SessionApplicationSnapshot {
                        application_id: supporting.application.id.clone(),
                        name: supporting.application.name.clone(),
                        role: crate::SessionApplicationRole::Supporting,
                        requirement: Some(supporting.requirement.clone()),
                        state: crate::SessionApplicationState::Pending,
                    })
                    .collect::<Vec<_>>();
                applications.push(crate::SessionApplicationSnapshot {
                    application_id: profile.primary_sim.id.clone(),
                    name: profile.primary_sim.name.clone(),
                    role: crate::SessionApplicationRole::PrimarySim,
                    requirement: None,
                    state: crate::SessionApplicationState::Pending,
                });

                let ordered_applications = self.ordered_session_applications(&profile)?;
                self.session = crate::SessionSnapshot {
                    state: crate::SessionState::Starting,
                    active_profile_id: Some(profile_id.clone()),
                    applications,
                    summary: None,
                };
                self.session_events.clear();
                for (index, application) in ordered_applications.iter().enumerate() {
                    self.prepare_elevated_session_launches(&ordered_applications, index)?;
                    let ownership = match self
                        .launch_or_adopt(&application.id, application.launch_recipe.clone())
                    {
                        Ok(ownership) => ownership,
                        Err(CoreError::ProcessRuntime(_))
                            if self.session.applications[index].requirement
                                == Some(crate::ApplicationRequirement::Optional) =>
                        {
                            self.session.applications[index].state =
                                crate::SessionApplicationState::Failed;
                            self.record_session_event(
                                &application.id,
                                crate::SessionEventKind::LaunchFailed,
                            );
                            continue;
                        }
                        Err(CoreError::ProcessRuntime(_)) => {
                            self.session.applications[index].state =
                                crate::SessionApplicationState::Failed;
                            self.record_session_event(
                                &application.id,
                                crate::SessionEventKind::LaunchFailed,
                            );
                            self.abort_session_startup()?;
                            return Ok(CommandOutcome::SessionStartFailed {
                                application_id: application.id.clone(),
                            });
                        }
                        Err(error) => return Err(error),
                    };
                    self.session.applications[index].state = match ownership {
                        ProcessOwnership::SessionOwned => crate::SessionApplicationState::Starting,
                        ProcessOwnership::PreExisting => {
                            crate::SessionApplicationState::RunningPreExisting
                        }
                    };
                    break;
                }

                Ok(CommandOutcome::SessionStartRequested { profile_id })
            }
            AppCommand::CancelStartup => {
                self.pending_process_confirmation = None;
                if self.session.state != crate::SessionState::Starting {
                    return Err(CoreError::InvalidSessionTransition {
                        current: self.session.state.clone(),
                        command: "Cancel Startup",
                    });
                }
                self.session.state = crate::SessionState::Cancelling;
                self.advance_session_cancellation()?;
                Ok(CommandOutcome::SessionCancellationRequested)
            }
            AppCommand::CloseSession => {
                self.pending_process_confirmation = None;
                if self.session.state != crate::SessionState::Active {
                    return Err(CoreError::InvalidSessionTransition {
                        current: self.session.state.clone(),
                        command: "Close Session",
                    });
                }
                self.session.state = crate::SessionState::Closing;
                Ok(CommandOutcome::SessionCloseRequested)
            }
            AppCommand::AcceptRecovery => {
                if self.session.state != crate::SessionState::RecoveryAvailable {
                    return Err(CoreError::InvalidSessionTransition {
                        current: self.session.state.clone(),
                        command: "Resume Recovery",
                    });
                }
                self.session.state = crate::SessionState::Active;
                Ok(CommandOutcome::RecoveryAccepted)
            }
            AppCommand::DismissRecovery => {
                if self.session.state != crate::SessionState::RecoveryAvailable {
                    return Err(CoreError::InvalidSessionTransition {
                        current: self.session.state.clone(),
                        command: "Dismiss Recovery",
                    });
                }
                for process in self.application_processes.values_mut() {
                    process.ownership = Some(ProcessOwnership::PreExisting);
                    process.status = ProcessStatus::RunningPreExisting;
                }
                for application in &mut self.session.applications {
                    application.state = crate::SessionApplicationState::Detached;
                }
                self.finish_session();
                Ok(CommandOutcome::RecoveryDismissed)
            }
            AppCommand::RequestWindowClose => {
                let action = match self.session.state {
                    crate::SessionState::Starting
                    | crate::SessionState::Cancelling
                    | crate::SessionState::Active
                    | crate::SessionState::Closing => crate::WindowCloseAction::HideToTray,
                    crate::SessionState::Idle | crate::SessionState::RecoveryAvailable => {
                        crate::WindowCloseAction::Exit
                    }
                };
                Ok(CommandOutcome::WindowCloseRequested { action })
            }
            AppCommand::RequestQuit { disposition } => {
                let action = match disposition {
                    crate::QuitDisposition::LeaveApplicationsRunning => {
                        for process in self.application_processes.values_mut() {
                            if process.identity.is_some() {
                                process.ownership = Some(ProcessOwnership::PreExisting);
                                process.status = ProcessStatus::RunningPreExisting;
                            }
                        }
                        for application in &mut self.session.applications {
                            application.state = crate::SessionApplicationState::Detached;
                        }
                        self.finish_session();
                        crate::QuitAction::ExitNow
                    }
                    crate::QuitDisposition::CloseSession => match self.session.state {
                        crate::SessionState::Idle => crate::QuitAction::ExitNow,
                        crate::SessionState::Starting => {
                            self.session.state = crate::SessionState::Cancelling;
                            crate::QuitAction::WaitForSessionClose
                        }
                        crate::SessionState::Active | crate::SessionState::RecoveryAvailable => {
                            self.session.state = crate::SessionState::Closing;
                            crate::QuitAction::WaitForSessionClose
                        }
                        crate::SessionState::Cancelling | crate::SessionState::Closing => {
                            crate::QuitAction::WaitForSessionClose
                        }
                    },
                };
                Ok(CommandOutcome::QuitRequested { action })
            }
            AppCommand::UpdateSettings { settings } => {
                self.settings_store.update_desktop(settings)?;
                Ok(CommandOutcome::SettingsUpdated)
            }
            AppCommand::ExportDiagnostics => {
                let snapshot = self.snapshot();
                let diagnostics = crate::DiagnosticExport {
                    schema_version: 1,
                    application_version: env!("CARGO_PKG_VERSION").to_owned(),
                    platform: std::env::consts::OS.to_owned(),
                    settings: snapshot.settings,
                    session_state: snapshot.session.state,
                    profile_count: snapshot.profiles.len(),
                    configured_application_count: self
                        .profile_library
                        .configured_application_count(),
                    recent_events: self.diagnostic_log.recent_entries(),
                    telemetry_upload: false,
                };
                Ok(CommandOutcome::DiagnosticsExported { diagnostics })
            }
            AppCommand::PrepareUpdateCheck {
                trigger,
                now_unix_seconds,
            } => {
                let automatic = trigger == crate::UpdateCheckTrigger::Automatic;
                let mut decision = self.update_advisor.prepare_check(
                    trigger,
                    now_unix_seconds,
                    &self.session.state,
                    self.settings_store.desktop(),
                    self.settings_store
                        .last_automatic_update_check_unix_seconds(),
                );
                if let crate::UpdateCheckDecision::Planned(plan) = &mut decision
                    && let Some(profile) = self.snapshot().selected_profile
                {
                    plan.applications = profile
                        .supporting_applications
                        .iter()
                        .map(|supporting| {
                            let application = &supporting.application;
                            crate::ApplicationUpdateTarget {
                                application_id: application.id.clone(),
                                name: application.name.clone(),
                                executable_path: match &application.launch_recipe.source {
                                    crate::LaunchSource::DirectExecutable { executable_path }
                                        if !application.path_needs_repair =>
                                    {
                                        Some(executable_path.clone())
                                    }
                                    _ => None,
                                },
                                provider: self
                                    .discovery_catalog
                                    .update_provider_for_name(&application.name),
                            }
                        })
                        .collect();
                }
                if automatic
                    && matches!(decision, crate::UpdateCheckDecision::Planned(_))
                    && let Err(error) = self
                        .settings_store
                        .record_automatic_update_check(now_unix_seconds)
                {
                    if let crate::UpdateCheckDecision::Planned(plan) = &decision {
                        let _ = self.update_advisor.cancel_check(&plan.request_id);
                    }
                    return Err(error);
                }
                Ok(CommandOutcome::UpdateCheckPrepared { decision })
            }
            AppCommand::CompleteUpdateCheck { result } => {
                self.update_advisor
                    .complete_check(result, &self.session.state)
                    .map_err(CoreError::InvalidUpdateCheck)?;
                Ok(CommandOutcome::UpdateCheckCompleted)
            }
            AppCommand::CancelUpdateCheck { request_id } => {
                if !self.update_advisor.cancel_check(&request_id) {
                    return Err(CoreError::InvalidUpdateCheck(
                        "the cancelled update check does not match the pending request".to_owned(),
                    ));
                }
                Ok(CommandOutcome::UpdateCheckCancelled)
            }
            AppCommand::PrepareFormationLapInstall => {
                let decision = if self.formation_lap_installing.is_some() {
                    crate::FormationLapInstallDecision::Deferred
                } else {
                    self.update_advisor
                        .prepare_formation_lap_install(&self.session.state)
                };
                if let crate::FormationLapInstallDecision::Ready { latest_version } = &decision {
                    self.formation_lap_installing = Some(latest_version.clone());
                }
                Ok(CommandOutcome::FormationLapInstallPrepared { decision })
            }
            AppCommand::CancelFormationLapInstall { expected_version } => {
                if self.formation_lap_installing.as_deref() != Some(expected_version.as_str()) {
                    return Err(CoreError::InvalidUpdateCheck(
                        "the update install lease does not match the checked version".to_owned(),
                    ));
                }
                self.formation_lap_installing = None;
                Ok(CommandOutcome::FormationLapInstallCancelled)
            }
            AppCommand::RefreshProcesses => {
                let application_ids = self
                    .application_processes
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>();
                for application_id in application_ids {
                    let Some((identity, previous_status)) = self
                        .application_processes
                        .get(&application_id)
                        .and_then(|process| {
                            process
                                .identity
                                .clone()
                                .map(|identity| (identity, process.status.clone()))
                        })
                    else {
                        continue;
                    };
                    if previous_status == ProcessStatus::Starting
                        && !self.post_start_ready_at.contains_key(&application_id)
                    {
                        let startup_timeout = self
                            .application_recipes
                            .get(&application_id)
                            .map(|recipe| {
                                Duration::from_secs(recipe.startup_timeout_seconds.into())
                            })
                            .unwrap_or_else(|| Duration::from_secs(30));
                        if self
                            .startup_started_at
                            .get(&application_id)
                            .is_some_and(|started_at| started_at.elapsed() >= startup_timeout)
                        {
                            self.application_processes
                                .get_mut(&application_id)
                                .expect("collected application Process should remain present")
                                .status = ProcessStatus::Failed;
                            self.record_session_event(
                                &application_id,
                                crate::SessionEventKind::LaunchFailed,
                            );
                            continue;
                        }
                    }
                    let observation = self.process_runtime.observe(&identity)?;
                    let observation_is_exit = matches!(
                        &observation,
                        ProcessObservation::Exited | ProcessObservation::Replaced { .. }
                    );
                    let session_was_active = self.session.state == crate::SessionState::Active;
                    let output = self.process_runtime.read_output(&identity)?;
                    let process = self
                        .application_processes
                        .get_mut(&application_id)
                        .expect("collected application Process should remain present");
                    process.output = if output.stdout.is_empty()
                        && output.stderr.is_empty()
                        && !output.truncated
                    {
                        None
                    } else {
                        Some(output)
                    };
                    match observation {
                        ProcessObservation::Running { responsiveness } => {
                            let failed_checks = self
                                .failed_responsiveness_checks
                                .entry(application_id.clone())
                                .or_default();
                            match responsiveness {
                                ProcessResponsiveness::NotResponsive => {
                                    *failed_checks = failed_checks.saturating_add(1);
                                }
                                ProcessResponsiveness::NotApplicable
                                | ProcessResponsiveness::Responsive => {
                                    *failed_checks = 0;
                                }
                            }
                            let post_start_delay = self
                                .application_recipes
                                .get(&application_id)
                                .map(|recipe| {
                                    Duration::from_millis(
                                        recipe.post_start_delay_milliseconds.into(),
                                    )
                                })
                                .unwrap_or_default();
                            let ready_at = *self
                                .post_start_ready_at
                                .entry(application_id.clone())
                                .or_insert_with(|| Instant::now() + post_start_delay);
                            process.status = if previous_status == ProcessStatus::Stopping {
                                ProcessStatus::Stopping
                            } else if Instant::now() < ready_at {
                                ProcessStatus::Starting
                            } else if *failed_checks >= 2 {
                                ProcessStatus::NotResponding
                            } else if process.ownership == Some(ProcessOwnership::PreExisting) {
                                ProcessStatus::RunningPreExisting
                            } else {
                                ProcessStatus::Running
                            };
                            if process.status != ProcessStatus::Starting {
                                self.startup_started_at.remove(&application_id);
                                self.post_start_ready_at.remove(&application_id);
                            }
                        }
                        ProcessObservation::Exited | ProcessObservation::Replaced { .. } => {
                            process.status = if previous_status == ProcessStatus::Starting {
                                ProcessStatus::Failed
                            } else {
                                ProcessStatus::Stopped
                            };
                            process.ownership = None;
                            process.identity = None;
                            self.failed_responsiveness_checks.remove(&application_id);
                            self.startup_started_at.remove(&application_id);
                            self.post_start_ready_at.remove(&application_id);
                        }
                    }
                    if observation_is_exit && session_was_active {
                        self.record_session_event(
                            &application_id,
                            crate::SessionEventKind::ApplicationExited,
                        );
                    }
                }
                self.sync_session_application_states();
                match self.session.state {
                    crate::SessionState::Starting => self.advance_session_startup()?,
                    crate::SessionState::Cancelling => self.advance_session_cancellation()?,
                    crate::SessionState::Active => self.begin_close_if_primary_exited(),
                    crate::SessionState::Closing => self.advance_session_close()?,
                    _ => {}
                }
                Ok(CommandOutcome::ProcessesRefreshed)
            }
            AppCommand::DiscoverApplications => Ok(CommandOutcome::ApplicationsDiscovered {
                discovery: self.discovery_catalog.snapshot(),
            }),
            AppCommand::RecommendApplications { primary_sim_id } => {
                Ok(CommandOutcome::ApplicationsRecommended {
                    recommendations: self.discovery_catalog.recommendations(&primary_sim_id),
                })
            }
            AppCommand::TestGameLaunch { profile_id } => {
                if self.session.state != crate::SessionState::Idle {
                    return Err(CoreError::InvalidSessionTransition {
                        current: self.session.state.clone(),
                        command: "Test Game Launch",
                    });
                }
                let mut profile = self
                    .profile_library
                    .profile(&profile_id)
                    .ok_or_else(|| CoreError::ProfileNotFound(profile_id.clone()))?;
                let primary_sim = self.resolved_primary_sim(&profile)?;
                let target = crate::launch_recipe::sanitized_target(&primary_sim.launch_recipe)
                    .map_err(CoreError::InvalidLaunchRecipe)?;
                self.launch_or_adopt(&primary_sim.id, primary_sim.launch_recipe.clone())?;
                let observed_identity = self
                    .application_processes
                    .get(&primary_sim.id)
                    .and_then(|process| process.identity.as_ref())
                    .cloned()
                    .ok_or_else(|| {
                        CoreError::InvalidLaunchRecipe(
                            "Test Game Launch did not observe a stable Process identity".to_owned(),
                        )
                    })?;
                let observed_process =
                    std::path::Path::new(&observed_identity.canonical_executable_path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .filter(|name| !name.trim().is_empty())
                        .ok_or_else(|| {
                            CoreError::InvalidLaunchRecipe(
                                "observed Process does not have a valid executable name".to_owned(),
                            )
                        })?
                        .to_owned();
                let monitored_process = primary_sim
                    .launch_recipe
                    .monitored_process
                    .as_deref()
                    .and_then(|name| std::path::Path::new(name).file_name())
                    .and_then(|name| name.to_str())
                    .filter(|name| !name.trim().is_empty())
                    .map(str::to_owned);
                let mut learned_identity = false;
                if profile
                    .primary_sim
                    .launch_recipe
                    .monitored_process
                    .is_none()
                {
                    profile.primary_sim.launch_recipe.monitored_process =
                        Some(observed_process.clone());
                    learned_identity = true;
                }
                if profile
                    .primary_sim
                    .launch_recipe
                    .monitored_executable_path
                    .is_none()
                {
                    profile.primary_sim.launch_recipe.monitored_executable_path =
                        Some(observed_identity.canonical_executable_path.clone());
                    learned_identity = true;
                }
                if learned_identity {
                    self.profile_library.save(profile.clone())?;
                }
                let diagnostic = crate::GameLaunchDiagnostic {
                    schema_version: 1,
                    profile_name: profile.name,
                    vr_enabled: profile.vr_enabled,
                    vr_launch_mode: profile
                        .vr_enabled
                        .then_some(profile.preferred_vr_launch_mode)
                        .flatten(),
                    target,
                    arguments: primary_sim.launch_recipe.arguments,
                    monitored_process,
                    observed_process,
                };
                self.game_launch_diagnostics.persist(&diagnostic)?;
                Ok(CommandOutcome::GameLaunchTested { diagnostic })
            }
        }
    }

    fn launch_or_adopt(
        &mut self,
        application_id: &str,
        launch_recipe: crate::LaunchRecipe,
    ) -> Result<ProcessOwnership, CoreError> {
        let (status, ownership, identity) = if launch_recipe.elevated {
            if let Some(prepared) = self.prepared_elevated_launches.remove(application_id) {
                match prepared {
                    PreparedElevatedLaunch::SessionOwned(identity) => (
                        ProcessStatus::Starting,
                        ProcessOwnership::SessionOwned,
                        identity,
                    ),
                    PreparedElevatedLaunch::PreExisting(identity) => (
                        ProcessStatus::RunningPreExisting,
                        ProcessOwnership::PreExisting,
                        identity,
                    ),
                    PreparedElevatedLaunch::Failed(message) => {
                        return Err(CoreError::PrivilegeBroker(PrivilegeBrokerError::new(
                            message,
                        )));
                    }
                }
            } else {
                let matches = self.process_runtime.matching_processes(&launch_recipe)?;
                if let Some(identity) = matches.into_iter().next() {
                    (
                        ProcessStatus::RunningPreExisting,
                        ProcessOwnership::PreExisting,
                        identity,
                    )
                } else {
                    let ownership = if launch_identity_is_unambiguous(&launch_recipe) {
                        ProcessOwnership::SessionOwned
                    } else {
                        ProcessOwnership::PreExisting
                    };
                    let identity =
                        self.execute_elevated_launch(application_id, &launch_recipe, &ownership)?;
                    (
                        if ownership == ProcessOwnership::SessionOwned {
                            ProcessStatus::Starting
                        } else {
                            ProcessStatus::RunningPreExisting
                        },
                        ownership,
                        identity,
                    )
                }
            }
        } else {
            let matches = self.process_runtime.matching_processes(&launch_recipe)?;
            if let Some(identity) = matches.into_iter().next() {
                (
                    ProcessStatus::RunningPreExisting,
                    ProcessOwnership::PreExisting,
                    identity,
                )
            } else {
                let identity = self.process_runtime.launch(&launch_recipe)?;
                let ownership = if launch_identity_is_unambiguous(&launch_recipe) {
                    ProcessOwnership::SessionOwned
                } else {
                    ProcessOwnership::PreExisting
                };
                (
                    if ownership == ProcessOwnership::SessionOwned {
                        ProcessStatus::Starting
                    } else {
                        ProcessStatus::RunningPreExisting
                    },
                    ownership,
                    identity,
                )
            }
        };
        self.application_processes.insert(
            application_id.to_owned(),
            ApplicationProcessSnapshot {
                application_id: application_id.to_owned(),
                status,
                ownership: Some(ownership.clone()),
                identity: Some(identity),
                output: None,
            },
        );
        self.failed_responsiveness_checks
            .insert(application_id.to_owned(), 0);
        self.application_recipes
            .insert(application_id.to_owned(), launch_recipe);
        if ownership == ProcessOwnership::SessionOwned {
            self.startup_started_at
                .insert(application_id.to_owned(), Instant::now());
            self.post_start_ready_at.remove(application_id);
        }

        Ok(ownership)
    }

    fn prepare_elevated_session_launches(
        &mut self,
        applications: &[crate::ProfileApplication],
        start_index: usize,
    ) -> Result<(), CoreError> {
        let Some(first) = applications.get(start_index) else {
            return Ok(());
        };
        if !first.launch_recipe.elevated || self.prepared_elevated_launches.contains_key(&first.id)
        {
            return Ok(());
        }

        let mut application_ids = Vec::new();
        let mut operations = Vec::new();
        for application in applications
            .iter()
            .skip(start_index)
            .take_while(|application| application.launch_recipe.elevated)
        {
            self.application_recipes
                .insert(application.id.clone(), application.launch_recipe.clone());
            if let Some(identity) = self
                .process_runtime
                .matching_processes(&application.launch_recipe)?
                .into_iter()
                .next()
            {
                self.prepared_elevated_launches.insert(
                    application.id.clone(),
                    PreparedElevatedLaunch::PreExisting(identity),
                );
                continue;
            }
            application_ids.push((
                application.id.clone(),
                launch_identity_is_unambiguous(&application.launch_recipe),
            ));
            operations.push(elevated_launch_operation(
                &application.launch_recipe,
                self.profile_library
                    .approved_executable_sha256(&application.id, false)?,
            )?);
        }
        if operations.is_empty() {
            return Ok(());
        }

        let session = &self.session;
        let journal = &self.session_journal;
        let application_processes = &mut self.application_processes;
        let acknowledgement_applications = &application_ids;
        let mut acknowledge = |operation_index: usize,
                               identity: &ProcessIdentity|
         -> Result<(), PrivilegeBrokerError> {
            let Some((application_id, identity_is_unambiguous)) =
                acknowledgement_applications.get(operation_index)
            else {
                return Err(PrivilegeBrokerError::new(
                    "helper acknowledged an unknown elevated launch",
                ));
            };
            let ownership = if *identity_is_unambiguous {
                ProcessOwnership::SessionOwned
            } else {
                ProcessOwnership::PreExisting
            };
            let previous = application_processes.insert(
                application_id.clone(),
                ApplicationProcessSnapshot {
                    application_id: application_id.clone(),
                    status: if ownership == ProcessOwnership::SessionOwned {
                        ProcessStatus::Starting
                    } else {
                        ProcessStatus::RunningPreExisting
                    },
                    ownership: Some(ownership),
                    identity: Some(identity.clone()),
                    output: None,
                },
            );
            let journal_processes = application_processes
                .values()
                .filter(|process| process.identity.is_some())
                .cloned()
                .collect::<Vec<_>>();
            if let Err(error) = journal.persist(session, &journal_processes) {
                if let Some(previous) = previous {
                    application_processes.insert(application_id.clone(), previous);
                } else {
                    application_processes.remove(application_id);
                }
                return Err(PrivilegeBrokerError::new(format!(
                    "elevated Process ownership could not be journaled: {error}"
                )));
            }
            Ok(())
        };
        let response = self
            .privilege_broker
            .execute_launch_batch(&operations, &mut acknowledge)?;
        if !response.accepted {
            return Err(CoreError::PrivilegeBroker(PrivilegeBrokerError::new(
                response
                    .error
                    .unwrap_or_else(|| "elevated helper rejected the launch batch".to_owned()),
            )));
        }
        if response.results.len() != application_ids.len() {
            return Err(CoreError::PrivilegeBroker(PrivilegeBrokerError::new(
                "elevated launch batch returned the wrong number of results",
            )));
        }
        for ((application_id, identity_is_unambiguous), result) in
            application_ids.into_iter().zip(response.results)
        {
            let prepared = match result {
                ElevatedOperationResult::Launched { process_identity }
                    if identity_is_unambiguous =>
                {
                    PreparedElevatedLaunch::SessionOwned(process_identity)
                }
                ElevatedOperationResult::Launched { process_identity } => {
                    PreparedElevatedLaunch::PreExisting(process_identity)
                }
                ElevatedOperationResult::Failed { message } => {
                    PreparedElevatedLaunch::Failed(message)
                }
                _ => PreparedElevatedLaunch::Failed(
                    "elevated launch returned an unexpected result".to_owned(),
                ),
            };
            self.prepared_elevated_launches
                .insert(application_id, prepared);
        }
        Ok(())
    }

    fn execute_elevated_launch(
        &mut self,
        application_id: &str,
        recipe: &crate::LaunchRecipe,
        ownership: &ProcessOwnership,
    ) -> Result<ProcessIdentity, CoreError> {
        let operation = elevated_launch_operation(
            recipe,
            self.profile_library
                .approved_executable_sha256(application_id, false)?,
        )?;
        let session = &self.session;
        let journal = &self.session_journal;
        let application_processes = &mut self.application_processes;
        let expected_application_id = application_id.to_owned();
        let expected_ownership = ownership.clone();
        let mut acknowledge = |operation_index: usize,
                               identity: &ProcessIdentity|
         -> Result<(), PrivilegeBrokerError> {
            if operation_index != 0 {
                return Err(PrivilegeBrokerError::new(
                    "helper acknowledged an unknown elevated launch",
                ));
            }
            let previous = application_processes.insert(
                expected_application_id.clone(),
                ApplicationProcessSnapshot {
                    application_id: expected_application_id.clone(),
                    status: if expected_ownership == ProcessOwnership::SessionOwned {
                        ProcessStatus::Starting
                    } else {
                        ProcessStatus::RunningPreExisting
                    },
                    ownership: Some(expected_ownership.clone()),
                    identity: Some(identity.clone()),
                    output: None,
                },
            );
            if session.state == crate::SessionState::Idle {
                return Ok(());
            }
            let journal_processes = application_processes
                .values()
                .filter(|process| process.identity.is_some())
                .cloned()
                .collect::<Vec<_>>();
            if let Err(error) = journal.persist(session, &journal_processes) {
                if let Some(previous) = previous {
                    application_processes.insert(expected_application_id.clone(), previous);
                } else {
                    application_processes.remove(&expected_application_id);
                }
                return Err(PrivilegeBrokerError::new(format!(
                    "elevated Process ownership could not be journaled: {error}"
                )));
            }
            Ok(())
        };
        let response = self
            .privilege_broker
            .execute_launch_batch(&[operation], &mut acknowledge)?;
        match response.results.into_iter().next() {
            Some(ElevatedOperationResult::Launched { process_identity }) => Ok(process_identity),
            Some(ElevatedOperationResult::Failed { message }) => Err(CoreError::PrivilegeBroker(
                PrivilegeBrokerError::new(message),
            )),
            _ => Err(CoreError::PrivilegeBroker(PrivilegeBrokerError::new(
                "elevated launch returned an unexpected result",
            ))),
        }
    }

    fn request_graceful_stop(
        &mut self,
        application_id: &str,
        identity: &ProcessIdentity,
        strategy: &crate::ShutdownStrategy,
        elevated: bool,
    ) -> Result<(crate::GracefulStopResult, bool), CoreError> {
        if !elevated {
            let requested = self
                .process_runtime
                .request_graceful_stop(identity, strategy)?;
            let exited = requested == crate::GracefulStopResult::Requested
                && self
                    .process_runtime
                    .wait_for_exit(identity, Duration::from_secs(5))?;
            return Ok((requested, exited));
        }

        let response = self
            .privilege_broker
            .execute(&[ElevatedOperation::GracefulStop {
                process_identity: identity.clone(),
                strategy: strategy.clone(),
                custom_stop_executable_sha256: matches!(
                    strategy,
                    crate::ShutdownStrategy::CustomStop { .. }
                )
                .then(|| {
                    self.profile_library
                        .approved_executable_sha256(application_id, true)
                })
                .transpose()?,
            }])?;
        match response.results.into_iter().next() {
            Some(ElevatedOperationResult::GracefulStopRequested { requested, exited }) => Ok((
                if requested {
                    crate::GracefulStopResult::Requested
                } else {
                    crate::GracefulStopResult::Unavailable
                },
                exited,
            )),
            Some(ElevatedOperationResult::Failed { message }) => Err(CoreError::PrivilegeBroker(
                PrivilegeBrokerError::new(message),
            )),
            _ => Err(CoreError::PrivilegeBroker(PrivilegeBrokerError::new(
                "elevated close returned an unexpected result",
            ))),
        }
    }

    fn force_stop_process(
        &mut self,
        identity: &ProcessIdentity,
        elevated: bool,
    ) -> Result<(), CoreError> {
        if !elevated {
            return self
                .process_runtime
                .force_stop(identity)
                .map_err(Into::into);
        }
        let response = self
            .privilege_broker
            .execute(&[ElevatedOperation::ForceTerminate {
                process_identity: identity.clone(),
            }])?;
        match response.results.into_iter().next() {
            Some(ElevatedOperationResult::ForceTerminated) => Ok(()),
            Some(ElevatedOperationResult::Failed { message }) => Err(CoreError::PrivilegeBroker(
                PrivilegeBrokerError::new(message),
            )),
            _ => Err(CoreError::PrivilegeBroker(PrivilegeBrokerError::new(
                "elevated termination returned an unexpected result",
            ))),
        }
    }

    fn ensure_active_profile_is_editable(&self, profile_id: &str) -> Result<(), CoreError> {
        if self.session.state != crate::SessionState::Idle
            && self.session.active_profile_id.as_deref() == Some(profile_id)
        {
            return Err(CoreError::InvalidSessionTransition {
                current: self.session.state.clone(),
                command: "Edit Active Profile",
            });
        }
        Ok(())
    }

    fn ensure_manual_lifecycle_is_available(&self, command: &'static str) -> Result<(), CoreError> {
        if !matches!(
            self.session.state,
            crate::SessionState::Idle | crate::SessionState::Active
        ) {
            return Err(CoreError::InvalidSessionTransition {
                current: self.session.state.clone(),
                command,
            });
        }
        Ok(())
    }

    fn ensure_force_stop_is_available(&self) -> Result<(), CoreError> {
        if !matches!(
            self.session.state,
            crate::SessionState::Idle | crate::SessionState::Active | crate::SessionState::Closing
        ) {
            return Err(CoreError::InvalidSessionTransition {
                current: self.session.state.clone(),
                command: "Force Stop Application",
            });
        }
        Ok(())
    }

    fn sync_session_journal(&self) -> Result<(), CoreError> {
        if self.session.state == crate::SessionState::Idle {
            self.session_journal.clear()
        } else {
            let application_processes = self
                .application_processes
                .values()
                .filter(|process| process.identity.is_some())
                .cloned()
                .collect::<Vec<_>>();
            self.session_journal
                .persist(&self.session, &application_processes)
        }
    }

    fn record_session_event(&mut self, application_id: &str, kind: crate::SessionEventKind) {
        if self
            .session_events
            .iter()
            .any(|event| event.application_id == application_id && event.kind == kind)
        {
            return;
        }
        let Some(application) = self
            .session
            .applications
            .iter()
            .find(|application| application.application_id == application_id)
        else {
            return;
        };
        self.session_events.push(crate::SessionEvent {
            application_id: application_id.to_owned(),
            name: application.name.clone(),
            kind,
        });
    }

    fn finish_session(&mut self) {
        self.session.summary = self
            .session
            .active_profile_id
            .clone()
            .filter(|_| !self.session_events.is_empty())
            .map(|profile_id| crate::SessionSummary {
                profile_id,
                events: self.session_events.clone(),
            });
        self.session.state = crate::SessionState::Idle;
        self.session.active_profile_id = None;
        self.prepared_elevated_launches.clear();
    }

    fn abort_session_startup(&mut self) -> Result<(), CoreError> {
        self.session.state = crate::SessionState::Cancelling;
        self.advance_session_cancellation()
    }

    fn advance_session_startup(&mut self) -> Result<(), CoreError> {
        if self.session.state != crate::SessionState::Starting {
            return Ok(());
        }

        let profile_id = self
            .session
            .active_profile_id
            .clone()
            .expect("a Starting Session should identify its Racing Profile");
        let profile = self
            .profile_library
            .profile(&profile_id)
            .ok_or_else(|| CoreError::ProfileNotFound(profile_id.clone()))?;
        let ordered_applications = self.ordered_session_applications(&profile)?;

        loop {
            if let Some(blocking_failure) = self.session.applications.iter().find(|application| {
                application.state == crate::SessionApplicationState::Failed
                    && application.requirement != Some(crate::ApplicationRequirement::Optional)
            }) {
                let application_id = blocking_failure.application_id.clone();
                self.record_session_event(&application_id, crate::SessionEventKind::LaunchFailed);
                self.abort_session_startup()?;
                return Ok(());
            }

            let Some(next_index) = self.session.applications.iter().position(|application| {
                application.state == crate::SessionApplicationState::Pending
            }) else {
                if self.session.applications.iter().all(|application| {
                    matches!(
                        application.state,
                        crate::SessionApplicationState::Running
                            | crate::SessionApplicationState::RunningPreExisting
                            | crate::SessionApplicationState::Failed
                    )
                }) {
                    self.session.state = crate::SessionState::Active;
                }
                return Ok(());
            };

            let prior_applications_are_ready =
                self.session.applications[..next_index]
                    .iter()
                    .all(|application| {
                        matches!(
                            application.state,
                            crate::SessionApplicationState::Running
                                | crate::SessionApplicationState::RunningPreExisting
                        ) || (application.state == crate::SessionApplicationState::Failed
                            && application.requirement
                                == Some(crate::ApplicationRequirement::Optional))
                    });
            if !prior_applications_are_ready {
                return Ok(());
            }

            let application = &ordered_applications[next_index];
            let application_id = application.id.clone();
            self.prepare_elevated_session_launches(&ordered_applications, next_index)?;
            let ownership =
                match self.launch_or_adopt(&application_id, application.launch_recipe.clone()) {
                    Ok(ownership) => ownership,
                    Err(CoreError::ProcessRuntime(_))
                        if self.session.applications[next_index].requirement
                            == Some(crate::ApplicationRequirement::Optional) =>
                    {
                        self.session.applications[next_index].state =
                            crate::SessionApplicationState::Failed;
                        self.record_session_event(
                            &application_id,
                            crate::SessionEventKind::LaunchFailed,
                        );
                        continue;
                    }
                    Err(CoreError::ProcessRuntime(_)) => {
                        self.session.applications[next_index].state =
                            crate::SessionApplicationState::Failed;
                        self.record_session_event(
                            &application_id,
                            crate::SessionEventKind::LaunchFailed,
                        );
                        self.abort_session_startup()?;
                        return Ok(());
                    }
                    Err(error) => return Err(error),
                };
            self.session.applications[next_index].state = match ownership {
                ProcessOwnership::SessionOwned => crate::SessionApplicationState::Starting,
                ProcessOwnership::PreExisting => crate::SessionApplicationState::RunningPreExisting,
            };
            if ownership == ProcessOwnership::SessionOwned {
                return Ok(());
            }
        }
    }

    fn sync_session_application_states(&mut self) {
        for application in &mut self.session.applications {
            let Some(process) = self.application_processes.get(&application.application_id) else {
                continue;
            };
            if application.state == crate::SessionApplicationState::Pending
                && process.identity.is_none()
            {
                continue;
            }
            application.state = match process.status {
                ProcessStatus::Starting => crate::SessionApplicationState::Starting,
                ProcessStatus::Running => crate::SessionApplicationState::Running,
                ProcessStatus::RunningPreExisting => {
                    crate::SessionApplicationState::RunningPreExisting
                }
                ProcessStatus::NotResponding => crate::SessionApplicationState::Running,
                ProcessStatus::Stopping => crate::SessionApplicationState::Stopping,
                ProcessStatus::Stopped => crate::SessionApplicationState::Stopped,
                ProcessStatus::Failed => crate::SessionApplicationState::Failed,
            };
        }
    }

    fn ordered_session_applications(
        &self,
        profile: &RacingProfile,
    ) -> Result<Vec<crate::ProfileApplication>, CoreError> {
        let primary_sim = self.resolved_primary_sim(profile)?;
        Ok(profile
            .supporting_applications
            .iter()
            .map(|supporting| supporting.application.clone())
            .chain(std::iter::once(primary_sim))
            .collect())
    }

    fn resolved_primary_sim(
        &self,
        profile: &RacingProfile,
    ) -> Result<crate::ProfileApplication, CoreError> {
        let mut primary_sim = profile.primary_sim.clone();
        primary_sim.launch_recipe = self
            .discovery_catalog
            .resolve_primary_launch_recipe(
                &primary_sim.launch_recipe,
                profile.vr_enabled,
                profile.preferred_vr_launch_mode.as_ref(),
            )
            .map_err(CoreError::InvalidLaunchRecipe)?;
        Ok(primary_sim)
    }

    fn begin_close_if_primary_exited(&mut self) {
        let primary_exited = self
            .session
            .applications
            .iter()
            .find(|application| application.role == crate::SessionApplicationRole::PrimarySim)
            .is_some_and(|application| {
                matches!(
                    application.state,
                    crate::SessionApplicationState::Stopped
                        | crate::SessionApplicationState::Failed
                )
            });
        if primary_exited {
            self.session.state = crate::SessionState::Closing;
        }
    }

    fn advance_session_close(&mut self) -> Result<(), CoreError> {
        if self.session.state != crate::SessionState::Closing {
            return Ok(());
        }
        let profile_id = self
            .session
            .active_profile_id
            .clone()
            .expect("a Closing Session should identify its Racing Profile");
        let profile = self
            .profile_library
            .profile(&profile_id)
            .ok_or_else(|| CoreError::ProfileNotFound(profile_id.clone()))?;
        let mut cleanup_order = vec![(profile.primary_sim.id.clone(), false)];
        cleanup_order.extend(
            profile
                .supporting_applications
                .iter()
                .rev()
                .map(|supporting| {
                    let preserve_steam_vr =
                        supporting.application.name.eq_ignore_ascii_case("SteamVR")
                            && !profile.close_session.stop_steam_vr;
                    (
                        supporting.application.id.clone(),
                        supporting.keep_running || preserve_steam_vr,
                    )
                }),
        );

        for (application_id, keep_running) in cleanup_order {
            let Some(index) = self
                .session
                .applications
                .iter()
                .position(|application| application.application_id == application_id)
            else {
                continue;
            };
            let Some(process) = self.application_processes.get(&application_id).cloned() else {
                continue;
            };
            if process.ownership == Some(ProcessOwnership::PreExisting) || keep_running {
                if let Some(process) = self.application_processes.get_mut(&application_id) {
                    process.ownership = Some(ProcessOwnership::PreExisting);
                    process.status = ProcessStatus::RunningPreExisting;
                }
                self.session.applications[index].state = crate::SessionApplicationState::Detached;
                continue;
            }
            if process.ownership != Some(ProcessOwnership::SessionOwned) {
                continue;
            }
            if process.status == ProcessStatus::Stopping {
                if self.pending_process_confirmation.is_none()
                    && let Some(identity) = process.identity
                {
                    self.request_process_confirmation(
                        application_id.clone(),
                        crate::ProcessConfirmationAction::SessionClose,
                        identity,
                        None,
                    );
                }
                return Ok(());
            }
            let Some(identity) = process.identity else {
                continue;
            };
            let recipe = self
                .application_recipes
                .get(&application_id)
                .cloned()
                .ok_or_else(|| CoreError::ApplicationNotFound(application_id.clone()))?;
            let (graceful, exited) = match self.request_graceful_stop(
                &application_id,
                &identity,
                &recipe.shutdown_strategy,
                recipe.elevated,
            ) {
                Ok(result) => result,
                Err(error) => {
                    self.application_processes
                        .get_mut(&application_id)
                        .expect("Session Process should remain present during cleanup")
                        .status = ProcessStatus::Stopping;
                    self.session.applications[index].state =
                        crate::SessionApplicationState::Stopping;
                    return Err(error);
                }
            };
            if graceful == crate::GracefulStopResult::Requested && exited {
                let process = self
                    .application_processes
                    .get_mut(&application_id)
                    .expect("Session Process should remain present during cleanup");
                process.status = ProcessStatus::Stopped;
                process.ownership = None;
                process.identity = None;
                self.session.applications[index].state = crate::SessionApplicationState::Stopped;
                self.failed_responsiveness_checks.remove(&application_id);
            } else {
                self.application_processes
                    .get_mut(&application_id)
                    .expect("Session Process should remain present during cleanup")
                    .status = ProcessStatus::Stopping;
                self.session.applications[index].state = crate::SessionApplicationState::Stopping;
                self.request_process_confirmation(
                    application_id.clone(),
                    crate::ProcessConfirmationAction::SessionClose,
                    identity,
                    None,
                );
                return Ok(());
            }
        }

        self.finish_session();
        Ok(())
    }

    fn advance_session_cancellation(&mut self) -> Result<(), CoreError> {
        if self.session.state != crate::SessionState::Cancelling {
            return Ok(());
        }

        if !self.cleanup_elevated_session_processes()? {
            return Ok(());
        }

        for index in (0..self.session.applications.len()).rev() {
            let application_id = self.session.applications[index].application_id.clone();
            let Some(process) = self.application_processes.get(&application_id).cloned() else {
                continue;
            };
            match process.ownership {
                Some(ProcessOwnership::PreExisting) => {
                    self.session.applications[index].state =
                        crate::SessionApplicationState::Detached;
                }
                Some(ProcessOwnership::SessionOwned) => {
                    let Some(identity) = process.identity else {
                        self.session.applications[index].state =
                            crate::SessionApplicationState::Stopped;
                        continue;
                    };
                    let recipe = self
                        .application_recipes
                        .get(&application_id)
                        .cloned()
                        .ok_or_else(|| CoreError::ApplicationNotFound(application_id.clone()))?;
                    let (graceful, exited) = self.request_graceful_stop(
                        &application_id,
                        &identity,
                        &recipe.shutdown_strategy,
                        recipe.elevated,
                    )?;
                    if graceful == crate::GracefulStopResult::Requested && exited {
                        let process = self
                            .application_processes
                            .get_mut(&application_id)
                            .expect("Session Process should remain present during cleanup");
                        process.status = ProcessStatus::Stopped;
                        process.ownership = None;
                        process.identity = None;
                        self.session.applications[index].state =
                            crate::SessionApplicationState::Stopped;
                        self.failed_responsiveness_checks.remove(&application_id);
                    } else {
                        self.application_processes
                            .get_mut(&application_id)
                            .expect("Session Process should remain present during cleanup")
                            .status = ProcessStatus::Stopping;
                        self.session.applications[index].state =
                            crate::SessionApplicationState::Stopping;
                        return Ok(());
                    }
                }
                None => {}
            }
        }

        self.finish_session();
        Ok(())
    }

    fn cleanup_elevated_session_processes(&mut self) -> Result<bool, CoreError> {
        let mut operation_targets = Vec::new();
        let mut detached_or_failed = Vec::new();
        for application in self.session.applications.iter().rev() {
            let application_id = &application.application_id;
            let recipe = self.application_recipes.get(application_id);
            if !recipe.is_some_and(|recipe| recipe.elevated) {
                continue;
            }
            if let Some(process) = self.application_processes.get(application_id)
                && process.ownership == Some(ProcessOwnership::SessionOwned)
                && let Some(identity) = process.identity.clone()
            {
                operation_targets.push((application_id.clone(), identity));
                continue;
            }
            match self.prepared_elevated_launches.get(application_id) {
                Some(PreparedElevatedLaunch::SessionOwned(identity)) => {
                    operation_targets.push((application_id.clone(), identity.clone()));
                }
                Some(PreparedElevatedLaunch::PreExisting(_)) => {
                    detached_or_failed.push((
                        application_id.clone(),
                        crate::SessionApplicationState::Detached,
                    ));
                }
                Some(PreparedElevatedLaunch::Failed(_)) => {
                    detached_or_failed.push((
                        application_id.clone(),
                        crate::SessionApplicationState::Failed,
                    ));
                }
                None => {}
            }
        }

        for (application_id, state) in detached_or_failed {
            self.prepared_elevated_launches.remove(&application_id);
            if let Some(application) = self
                .session
                .applications
                .iter_mut()
                .find(|application| application.application_id == application_id)
            {
                application.state = state;
            }
        }
        if operation_targets.is_empty() {
            return Ok(true);
        }

        let operations = operation_targets
            .iter()
            .map(|(application_id, identity)| {
                let strategy = self
                    .application_recipes
                    .get(application_id)
                    .map(|recipe| recipe.shutdown_strategy.clone())
                    .ok_or_else(|| CoreError::ApplicationNotFound(application_id.clone()))?;
                Ok(ElevatedOperation::GracefulStop {
                    process_identity: identity.clone(),
                    strategy,
                    custom_stop_executable_sha256: matches!(
                        self.application_recipes
                            .get(application_id)
                            .map(|recipe| &recipe.shutdown_strategy),
                        Some(crate::ShutdownStrategy::CustomStop { .. })
                    )
                    .then(|| {
                        self.profile_library
                            .approved_executable_sha256(application_id, true)
                    })
                    .transpose()?,
                })
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        let response = self.privilege_broker.execute(&operations)?;
        if response.results.len() != operation_targets.len() {
            return Err(CoreError::PrivilegeBroker(PrivilegeBrokerError::new(
                "elevated cancellation returned the wrong number of results",
            )));
        }

        let mut all_stopped = true;
        for ((application_id, identity), result) in
            operation_targets.into_iter().zip(response.results)
        {
            let stopped = matches!(
                result,
                ElevatedOperationResult::GracefulStopRequested {
                    requested: true,
                    exited: true,
                }
            );
            self.prepared_elevated_launches.remove(&application_id);
            let session_application = self
                .session
                .applications
                .iter_mut()
                .find(|application| application.application_id == application_id)
                .expect("an elevated cleanup target should remain in the Session");
            if stopped {
                if let Some(process) = self.application_processes.get_mut(&application_id) {
                    process.status = ProcessStatus::Stopped;
                    process.ownership = None;
                    process.identity = None;
                }
                session_application.state = crate::SessionApplicationState::Stopped;
                self.failed_responsiveness_checks.remove(&application_id);
            } else {
                self.application_processes.insert(
                    application_id.clone(),
                    ApplicationProcessSnapshot {
                        application_id: application_id.clone(),
                        status: ProcessStatus::Stopping,
                        ownership: Some(ProcessOwnership::SessionOwned),
                        identity: Some(identity),
                        output: None,
                    },
                );
                session_application.state = crate::SessionApplicationState::Stopping;
                all_stopped = false;
            }
        }
        Ok(all_stopped)
    }
}

fn recovery_identity_matches_recipe(
    identity: &ProcessIdentity,
    recipe: &crate::LaunchRecipe,
) -> bool {
    let configured_path = recipe
        .monitored_executable_path
        .as_deref()
        .or(match &recipe.source {
            crate::LaunchSource::DirectExecutable { executable_path } => Some(executable_path),
            crate::LaunchSource::Steam { .. } => None,
        });
    configured_path
        .and_then(|path| std::path::Path::new(path).canonicalize().ok())
        .is_some_and(|path| {
            path.to_string_lossy()
                .eq_ignore_ascii_case(&identity.canonical_executable_path)
        })
}

fn elevated_launch_operation(
    recipe: &crate::LaunchRecipe,
    executable_sha256: String,
) -> Result<ElevatedOperation, CoreError> {
    let crate::LaunchSource::DirectExecutable { executable_path } = &recipe.source else {
        return Err(CoreError::InvalidLaunchRecipe(
            "Steam launches cannot request elevation".to_owned(),
        ));
    };
    let executable_path = crate::privilege_protocol::canonical_executable_path(executable_path)
        .map_err(|error| CoreError::InvalidLaunchRecipe(error.to_string()))?;
    let working_directory = recipe
        .working_directory
        .as_deref()
        .map(std::path::Path::new)
        .map(std::path::Path::canonicalize)
        .transpose()
        .map_err(|error| {
            CoreError::InvalidLaunchRecipe(format!(
                "elevated working directory is not accessible: {error}"
            ))
        })?
        .map(|path| path.to_string_lossy().into_owned());
    Ok(ElevatedOperation::Launch {
        executable_path,
        executable_sha256,
        arguments: recipe.arguments.clone(),
        working_directory,
        monitored_process: recipe.monitored_process.clone(),
        monitored_executable_path: recipe.monitored_executable_path.clone(),
        console_visibility: recipe.console_visibility.clone(),
        startup_timeout_seconds: recipe.startup_timeout_seconds,
    })
}

fn launch_identity_is_unambiguous(recipe: &crate::LaunchRecipe) -> bool {
    recipe.monitored_process.is_none() || recipe.monitored_executable_path.is_some()
}
