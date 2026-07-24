use crate::{
    AppSnapshot, ApplicationProcessSnapshot, ProcessOwnership, ProcessRuntime, ProcessRuntimeError,
    ProcessStatus, ProfileLibrary, RacingProfile, SettingsStore, WindowsProcessRuntime,
};
use std::{collections::BTreeMap, error::Error, fmt, io};

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

/// Owns authoritative Racing Profile and Session state.
pub struct FormationLapCore {
    profile_library: ProfileLibrary,
    settings_store: SettingsStore,
    process_runtime: Box<dyn ProcessRuntime>,
    application_processes: BTreeMap<String, ApplicationProcessSnapshot>,
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
                let matches = self
                    .process_runtime
                    .matching_processes(&application.launch_recipe)?;

                let (status, ownership, identity, outcome) =
                    if let Some(identity) = matches.into_iter().next() {
                        (
                            ProcessStatus::RunningPreExisting,
                            ProcessOwnership::PreExisting,
                            identity,
                            CommandOutcome::ApplicationAlreadyRunning {
                                application_id: application_id.clone(),
                            },
                        )
                    } else {
                        (
                            ProcessStatus::Starting,
                            ProcessOwnership::SessionOwned,
                            self.process_runtime.launch(&application.launch_recipe)?,
                            CommandOutcome::ApplicationStartRequested {
                                application_id: application_id.clone(),
                            },
                        )
                    };
                self.application_processes.insert(
                    application_id.clone(),
                    ApplicationProcessSnapshot {
                        application_id: application_id.clone(),
                        status,
                        ownership: Some(ownership),
                        identity: Some(identity),
                    },
                );

                Ok(outcome)
            }
        }
    }
}
