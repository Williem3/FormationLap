use crate::{LaunchRecipe, ProcessIdentity};
use std::{error::Error, fmt};

/// Failure reported by a ProcessRuntime adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessRuntimeError {
    message: String,
}

impl ProcessRuntimeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ProcessRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ProcessRuntimeError {}

/// Observes and controls local processes without deciding Session policy.
pub trait ProcessRuntime: Send {
    fn matching_processes(
        &mut self,
        recipe: &LaunchRecipe,
    ) -> Result<Vec<ProcessIdentity>, ProcessRuntimeError>;

    fn launch(&mut self, recipe: &LaunchRecipe) -> Result<ProcessIdentity, ProcessRuntimeError>;
}

/// Production ProcessRuntime adapter for Windows.
#[derive(Default)]
pub struct WindowsProcessRuntime;

impl WindowsProcessRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl ProcessRuntime for WindowsProcessRuntime {
    fn matching_processes(
        &mut self,
        _recipe: &LaunchRecipe,
    ) -> Result<Vec<ProcessIdentity>, ProcessRuntimeError> {
        Err(ProcessRuntimeError::new(
            "Windows process observation is not implemented yet",
        ))
    }

    fn launch(&mut self, _recipe: &LaunchRecipe) -> Result<ProcessIdentity, ProcessRuntimeError> {
        Err(ProcessRuntimeError::new(
            "Windows process launch is not implemented yet",
        ))
    }
}
