use crate::{AppSnapshot, ProfileLibrary};
use std::{error::Error, fmt, io};

/// User intent accepted by FormationLapCore.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppCommand {
    CreateProfile {
        name: String,
        primary_sim_name: String,
    },
}

/// Observable result of a completed FormationLapCore command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandOutcome {
    ProfileCreated { profile_id: String },
}

#[derive(Debug)]
pub enum CoreError {
    Storage(io::Error),
    InvalidProfileDocument(serde_json::Error),
    UnsupportedProfileSchema(u32),
}

impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => write!(formatter, "profile storage failed: {error}"),
            Self::InvalidProfileDocument(error) => {
                write!(formatter, "profile document is invalid: {error}")
            }
            Self::UnsupportedProfileSchema(version) => {
                write!(
                    formatter,
                    "profile schema version {version} is not supported"
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
            Self::UnsupportedProfileSchema(_) => None,
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

/// Owns authoritative Racing Profile and Session state.
pub struct FormationLapCore {
    profile_library: ProfileLibrary,
}

impl FormationLapCore {
    pub fn open(storage_root: impl AsRef<std::path::Path>) -> Result<Self, CoreError> {
        Ok(Self {
            profile_library: ProfileLibrary::open(storage_root)?,
        })
    }

    pub fn snapshot(&self) -> AppSnapshot {
        let mut snapshot = AppSnapshot::foundation();
        snapshot.profiles = self.profile_library.summaries();
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
        }
    }
}
