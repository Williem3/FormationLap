use crate::discovery_catalog::{DiscoveryCatalog, DiscoveryCatalogError};
use crate::{
    AppSnapshot, ApplicationProcessSnapshot, ProcessObservation, ProcessOwnership,
    ProcessResponsiveness, ProcessRuntime, ProcessRuntimeError, ProcessStatus, ProfileLibrary,
    RacingProfile, SettingsStore, WindowsProcessRuntime,
};
use std::{collections::BTreeMap, error::Error, fmt, io, time::Duration};

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
    DiscoverApplications,
    RefreshProcesses,
}

/// Observable result of a completed FormationLapCore command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandOutcome {
    ProfileCreated { profile_id: String },
    ProfileUpdated { profile_id: String },
    ProfileDeleted { profile_id: String },
    ProfileSelected { profile_id: String },
    ProfileExported { document: String },
    ApplicationStartRequested { application_id: String },
    ApplicationAlreadyRunning { application_id: String },
    ApplicationStopped { application_id: String },
    ApplicationRestarted { application_id: String },
    PreExistingControlConfirmationRequired { application_id: String },
    ForceStopConfirmationRequired { application_id: String },
    ApplicationsDiscovered { discovery: crate::DiscoverySnapshot },
    ProcessesRefreshed,
}

#[derive(Debug)]
pub enum CoreError {
    Storage(io::Error),
    InvalidProfileDocument(serde_json::Error),
    InvalidSettingsDocument(serde_json::Error),
    InvalidProfileName(&'static str),
    ProfileNotFound(String),
    ApplicationNotFound(String),
    ProcessRuntime(ProcessRuntimeError),
    DiscoveryCatalog(DiscoveryCatalogError),
    UnsupportedProfileSchema(u32),
    UnsupportedSettingsSchema(u32),
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
            Self::DiscoveryCatalog(error) => write!(formatter, "catalog discovery failed: {error}"),
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
        }
    }
}

impl Error for CoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            Self::InvalidProfileDocument(error) => Some(error),
            Self::InvalidSettingsDocument(error) => Some(error),
            Self::InvalidProfileName(_)
            | Self::ProfileNotFound(_)
            | Self::ApplicationNotFound(_)
            | Self::UnsupportedProfileSchema(_)
            | Self::UnsupportedSettingsSchema(_) => None,
            Self::ProcessRuntime(error) => Some(error),
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
    application_processes: BTreeMap<String, ApplicationProcessSnapshot>,
    failed_responsiveness_checks: BTreeMap<String, u8>,
    application_recipes: BTreeMap<String, crate::LaunchRecipe>,
    pending_restarts: BTreeMap<String, crate::LaunchRecipe>,
    discovery_catalog: DiscoveryCatalog,
}

impl FormationLapCore {
    pub fn open(storage_root: impl AsRef<std::path::Path>) -> Result<Self, CoreError> {
        Self::open_with_runtime(storage_root, WindowsProcessRuntime::new())
    }

    pub fn open_with_runtime(
        storage_root: impl AsRef<std::path::Path>,
        process_runtime: impl ProcessRuntime + 'static,
    ) -> Result<Self, CoreError> {
        let storage_root = storage_root.as_ref();
        Ok(Self {
            profile_library: ProfileLibrary::open(storage_root)?,
            settings_store: SettingsStore::open(storage_root)?,
            process_runtime: Box::new(process_runtime),
            application_processes: BTreeMap::new(),
            failed_responsiveness_checks: BTreeMap::new(),
            application_recipes: BTreeMap::new(),
            pending_restarts: BTreeMap::new(),
            discovery_catalog: DiscoveryCatalog::bundled()?,
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
        snapshot
    }

    pub fn execute(&mut self, command: AppCommand) -> Result<CommandOutcome, CoreError> {
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
                self.profile_library
                    .edit(&profile_id, name, primary_sim_name)?;
                Ok(CommandOutcome::ProfileUpdated { profile_id })
            }
            AppCommand::DeleteProfile { profile_id } => {
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
                let profile = self
                    .profile_library
                    .profile(&profile_id)
                    .ok_or_else(|| CoreError::ProfileNotFound(profile_id.clone()))?;
                let application = if profile.primary_sim.id == application_id {
                    Some(&profile.primary_sim)
                } else {
                    profile
                        .supporting_applications
                        .iter()
                        .map(|supporting| &supporting.application)
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
                    .map(|recipe| &recipe.shutdown_strategy)
                    .ok_or_else(|| CoreError::ApplicationNotFound(application_id.clone()))?;
                let graceful = self
                    .process_runtime
                    .request_graceful_stop(&identity, strategy)?;
                self.application_processes
                    .get_mut(&application_id)
                    .expect("application Process should remain present")
                    .status = ProcessStatus::Stopping;

                if graceful == crate::GracefulStopResult::Requested
                    && self
                        .process_runtime
                        .wait_for_exit(&identity, Duration::from_secs(5))?
                {
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
                self.process_runtime.force_stop(&identity)?;
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
                let profile = self
                    .profile_library
                    .profile(&profile_id)
                    .ok_or_else(|| CoreError::ProfileNotFound(profile_id.clone()))?;
                let application = if profile.primary_sim.id == application_id {
                    Some(&profile.primary_sim)
                } else {
                    profile
                        .supporting_applications
                        .iter()
                        .map(|supporting| &supporting.application)
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
                let graceful = self
                    .process_runtime
                    .request_graceful_stop(&identity, &recipe.shutdown_strategy)?;
                self.application_processes
                    .get_mut(&application_id)
                    .expect("application Process should remain present")
                    .status = ProcessStatus::Stopping;

                if graceful == crate::GracefulStopResult::Requested
                    && self
                        .process_runtime
                        .wait_for_exit(&identity, Duration::from_secs(5))?
                {
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
                    let observation = self.process_runtime.observe(&identity)?;
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
                            process.status = if *failed_checks >= 2 {
                                ProcessStatus::NotResponding
                            } else if process.ownership == Some(ProcessOwnership::PreExisting) {
                                ProcessStatus::RunningPreExisting
                            } else {
                                ProcessStatus::Running
                            };
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
                        }
                    }
                }
                Ok(CommandOutcome::ProcessesRefreshed)
            }
            AppCommand::DiscoverApplications => Ok(CommandOutcome::ApplicationsDiscovered {
                discovery: self.discovery_catalog.snapshot(),
            }),
        }
    }

    fn launch_or_adopt(
        &mut self,
        application_id: &str,
        launch_recipe: crate::LaunchRecipe,
    ) -> Result<ProcessOwnership, CoreError> {
        let matches = self.process_runtime.matching_processes(&launch_recipe)?;
        let (status, ownership, identity) = if let Some(identity) = matches.into_iter().next() {
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

        Ok(ownership)
    }
}
