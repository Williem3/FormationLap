use crate::discovery_catalog::{DiscoveryCatalog, DiscoveryCatalogError, TargetedDiscoverySources};
use crate::game_launch_diagnostics::GameLaunchDiagnostics;
use crate::{
    AppSnapshot, ApplicationProcessSnapshot, DevelopmentPrivilegeBroker, ElevatedOperation,
    ElevatedOperationResult, PrivilegeBroker, PrivilegeBrokerError, ProcessIdentity,
    ProcessObservation, ProcessOwnership, ProcessResponsiveness, ProcessRuntime,
    ProcessRuntimeError, ProcessStatus, ProfileLibrary, RacingProfile, SettingsStore,
    WindowsPrivilegeBroker, WindowsProcessRuntime, session_journal::SessionJournal,
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
        name: String,
        primary_sim_name: String,
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
    StartApplication {
        profile_id: String,
        application_id: String,
    },
    ExitApplication {
        application_id: String,
        pre_existing_confirmed: bool,
    },
    ForceStopApplication {
        application_id: String,
        pre_existing_confirmed: bool,
        force_confirmed: bool,
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
    RefreshProcesses,
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
    ApplicationNotFound(String),
    ProcessRuntime(ProcessRuntimeError),
    PrivilegeBroker(PrivilegeBrokerError),
    DiscoveryCatalog(DiscoveryCatalogError),
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
            Self::ApplicationNotFound(application_id) => {
                write!(formatter, "application {application_id} was not found")
            }
            Self::ProcessRuntime(error) => write!(formatter, "process runtime failed: {error}"),
            Self::PrivilegeBroker(error) => {
                write!(formatter, "privileged operation failed: {error}")
            }
            Self::DiscoveryCatalog(error) => write!(formatter, "catalog discovery failed: {error}"),
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
            | Self::ProfileNotFound(_)
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
    process_runtime: Box<dyn ProcessRuntime>,
    privilege_broker: Box<dyn PrivilegeBroker>,
    application_processes: BTreeMap<String, ApplicationProcessSnapshot>,
    failed_responsiveness_checks: BTreeMap<String, u8>,
    application_recipes: BTreeMap<String, crate::LaunchRecipe>,
    pending_restarts: BTreeMap<String, crate::LaunchRecipe>,
    prepared_elevated_launches: BTreeMap<String, PreparedElevatedLaunch>,
    startup_started_at: BTreeMap<String, Instant>,
    post_start_ready_at: BTreeMap<String, Instant>,
    session_events: Vec<crate::SessionEvent>,
    session_journal: SessionJournal,
    discovery_catalog: DiscoveryCatalog,
    game_launch_diagnostics: GameLaunchDiagnostics,
    session: crate::SessionSnapshot,
}

#[derive(Clone)]
enum PreparedElevatedLaunch {
    SessionOwned(ProcessIdentity),
    PreExisting(ProcessIdentity),
    Failed(String),
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
            for mut process in recovered.application_processes {
                let Some(identity) = process.identity.as_ref() else {
                    continue;
                };
                if matches!(
                    process_runtime.observe(identity),
                    Ok(ProcessObservation::Running { .. })
                ) {
                    process.status = if process.ownership == Some(ProcessOwnership::PreExisting) {
                        ProcessStatus::RunningPreExisting
                    } else {
                        ProcessStatus::Running
                    };
                    application_processes.insert(process.application_id.clone(), process);
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
                if let Some(profile) = session
                    .active_profile_id
                    .as_deref()
                    .and_then(|profile_id| profile_library.profile(profile_id))
                {
                    for application in profile
                        .supporting_applications
                        .iter()
                        .map(|supporting| &supporting.application)
                        .chain(std::iter::once(&profile.primary_sim))
                    {
                        application_recipes
                            .insert(application.id.clone(), application.launch_recipe.clone());
                    }
                }
            } else {
                session_journal.clear()?;
            }
        }
        Ok(Self {
            profile_library,
            settings_store: SettingsStore::open(storage_root)?,
            process_runtime: Box::new(process_runtime),
            privilege_broker: Box::new(privilege_broker),
            application_processes,
            failed_responsiveness_checks: BTreeMap::new(),
            application_recipes,
            pending_restarts: BTreeMap::new(),
            prepared_elevated_launches: BTreeMap::new(),
            startup_started_at: BTreeMap::new(),
            post_start_ready_at: BTreeMap::new(),
            session_events: Vec::new(),
            session_journal,
            discovery_catalog: DiscoveryCatalog::bundled_with_sources(discovery_sources)?,
            game_launch_diagnostics: GameLaunchDiagnostics::open(storage_root)?,
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
        snapshot.application_processes = self.application_processes.values().cloned().collect();
        snapshot.session = self.session.clone();
        snapshot
    }

    pub fn execute(&mut self, command: AppCommand) -> Result<CommandOutcome, CoreError> {
        let outcome = self.execute_inner(command)?;
        self.sync_session_journal()?;
        Ok(outcome)
    }

    fn execute_inner(&mut self, command: AppCommand) -> Result<CommandOutcome, CoreError> {
        match command {
            AppCommand::CreateProfile {
                name,
                primary_sim_name,
            } => {
                let profile_id = self.profile_library.create(name, primary_sim_name)?;
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
                let (graceful, exited) =
                    self.request_graceful_stop(&identity, &strategy.0, strategy.1)?;
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
                    Ok(CommandOutcome::ForceStopConfirmationRequired { application_id })
                }
            }
            AppCommand::ForceStopApplication {
                application_id,
                pre_existing_confirmed,
                force_confirmed,
            } => {
                self.ensure_manual_lifecycle_is_available("Force Stop Application")?;
                if !force_confirmed {
                    return Ok(CommandOutcome::ForceStopConfirmationRequired { application_id });
                }
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
                let elevated = self
                    .application_recipes
                    .get(&application_id)
                    .is_some_and(|recipe| recipe.elevated);
                self.force_stop_process(&identity, elevated)?;
                let process = self
                    .application_processes
                    .get_mut(&application_id)
                    .expect("application Process should remain present");
                process.status = ProcessStatus::Stopped;
                process.ownership = None;
                process.identity = None;
                self.failed_responsiveness_checks.remove(&application_id);
                if let Some(recipe) = self.pending_restarts.remove(&application_id) {
                    let ownership = self.launch_or_adopt(&application_id, recipe)?;
                    Ok(if ownership == ProcessOwnership::PreExisting {
                        CommandOutcome::ApplicationAlreadyRunning { application_id }
                    } else {
                        CommandOutcome::ApplicationRestarted { application_id }
                    })
                } else {
                    Ok(CommandOutcome::ApplicationStopped { application_id })
                }
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
                    self.pending_restarts.insert(application_id.clone(), recipe);
                    Ok(CommandOutcome::ForceStopConfirmationRequired { application_id })
                }
            }
            AppCommand::StartSession { profile_id } => {
                if self.session.state != crate::SessionState::Idle {
                    return Err(CoreError::InvalidSessionTransition {
                        current: self.session.state.clone(),
                        command: "Start Session",
                    });
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
                self.prepare_elevated_session_launches(&ordered_applications)?;
                self.session = crate::SessionSnapshot {
                    state: crate::SessionState::Starting,
                    active_profile_id: Some(profile_id.clone()),
                    applications,
                    summary: None,
                };
                self.session_events.clear();
                for (index, application) in ordered_applications.iter().enumerate() {
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
                if self.session.state != crate::SessionState::Starting {
                    return Err(CoreError::InvalidSessionTransition {
                        current: self.session.state.clone(),
                        command: "Cancel Startup",
                    });
                }
                self.session.state = crate::SessionState::Cancelling;
                Ok(CommandOutcome::SessionCancellationRequested)
            }
            AppCommand::CloseSession => {
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
                            process.status = if Instant::now() < ready_at {
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
                let observed_process = self
                    .application_processes
                    .get(&primary_sim.id)
                    .and_then(|process| process.identity.as_ref())
                    .and_then(|identity| {
                        std::path::Path::new(&identity.canonical_executable_path)
                            .file_name()
                            .and_then(|name| name.to_str())
                    })
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
                if profile
                    .primary_sim
                    .launch_recipe
                    .monitored_process
                    .is_none()
                {
                    profile.primary_sim.launch_recipe.monitored_process =
                        Some(observed_process.clone());
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
                    (
                        ProcessStatus::Starting,
                        ProcessOwnership::SessionOwned,
                        self.execute_elevated_launch(&launch_recipe)?,
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
                (
                    ProcessStatus::Starting,
                    ProcessOwnership::SessionOwned,
                    self.process_runtime.launch(&launch_recipe)?,
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
    ) -> Result<(), CoreError> {
        self.prepared_elevated_launches.clear();
        let mut application_ids = Vec::new();
        let mut operations = Vec::new();
        for application in applications
            .iter()
            .filter(|application| application.launch_recipe.elevated)
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
            application_ids.push(application.id.clone());
            operations.push(elevated_launch_operation(&application.launch_recipe)?);
        }
        if operations.is_empty() {
            return Ok(());
        }

        let response = self.privilege_broker.execute(&operations)?;
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
        for (application_id, result) in application_ids.into_iter().zip(response.results) {
            let prepared = match result {
                ElevatedOperationResult::Launched { process_identity } => {
                    PreparedElevatedLaunch::SessionOwned(process_identity)
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
        recipe: &crate::LaunchRecipe,
    ) -> Result<ProcessIdentity, CoreError> {
        let response = self
            .privilege_broker
            .execute(&[elevated_launch_operation(recipe)?])?;
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
            let Some(identity) = process.identity else {
                continue;
            };
            let recipe = self
                .application_recipes
                .get(&application_id)
                .cloned()
                .ok_or_else(|| CoreError::ApplicationNotFound(application_id.clone()))?;
            let (graceful, exited) =
                self.request_graceful_stop(&identity, &recipe.shutdown_strategy, recipe.elevated)?;
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

fn elevated_launch_operation(recipe: &crate::LaunchRecipe) -> Result<ElevatedOperation, CoreError> {
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
        arguments: recipe.arguments.clone(),
        working_directory,
        monitored_process: recipe.monitored_process.clone(),
        console_visibility: recipe.console_visibility.clone(),
        startup_timeout_seconds: recipe.startup_timeout_seconds,
    })
}
