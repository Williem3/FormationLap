use crate::{ConsoleVisibility, ProcessIdentity, ShutdownStrategy};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub const ELEVATED_HELPER_PROTOCOL_VERSION: u16 = 2;
pub const MAX_ELEVATED_OPERATIONS: usize = 32;
pub const MAX_ELEVATED_ARGUMENTS: usize = 32;
pub const MAX_ELEVATED_ARGUMENT_BYTES: usize = 16_384;
pub const MAX_HELPER_MESSAGE_BYTES: usize = 65_536;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ElevatedHelperRequest {
    pub protocol_version: u16,
    pub parent_identity: ProcessIdentity,
    pub nonce: String,
    pub current_user_id: String,
    pub operations: Vec<ElevatedOperation>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ElevatedOperation {
    Launch {
        executable_path: String,
        arguments: Vec<String>,
        working_directory: Option<String>,
        monitored_process: Option<String>,
        monitored_executable_path: Option<String>,
        console_visibility: ConsoleVisibility,
        startup_timeout_seconds: u32,
    },
    GracefulStop {
        process_identity: ProcessIdentity,
        strategy: ShutdownStrategy,
    },
    ForceTerminate {
        process_identity: ProcessIdentity,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ElevatedHelperResponse {
    pub protocol_version: u16,
    pub nonce: String,
    pub accepted: bool,
    pub error: Option<String>,
    pub results: Vec<ElevatedOperationResult>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ElevatedOwnershipOffer {
    pub protocol_version: u16,
    pub nonce: String,
    pub operation_index: usize,
    pub process_identity: ProcessIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ElevatedOwnershipAcknowledgement {
    pub protocol_version: u16,
    pub nonce: String,
    pub operation_index: usize,
    pub process_identity: ProcessIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ElevatedOperationResult {
    Launched { process_identity: ProcessIdentity },
    GracefulStopRequested { requested: bool, exited: bool },
    ForceTerminated,
    Failed { message: String },
}

/// Facts observed independently by the elevated helper before request validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HelperValidationContext {
    pub current_user_id: String,
    pub parent_identity: ProcessIdentity,
    pub helper_process_id: u32,
    pub operation_process_identities: Vec<ProcessIdentity>,
    pub same_interactive_session: bool,
    pub expected_application_path: bool,
    pub release_identity_verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HelperProtocolError {
    MessageTooLarge { maximum: usize, received: usize },
    InvalidDocument(String),
    VersionMismatch { expected: u16, received: u16 },
    InvalidNonce,
    ReplayedNonce,
    WrongUser,
    WrongParentIdentity,
    WrongInteractiveSession,
    UnexpectedApplicationPath,
    UnverifiedReleaseIdentity,
    EmptyBatch,
    BatchTooLarge { maximum: usize, received: usize },
    NonCanonicalPath(String),
    InvalidExecutable(String),
    InvalidArguments(String),
    WrongProcessIdentity(u32),
    ProtectedProcess(u32),
}

impl fmt::Display for HelperProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MessageTooLarge { maximum, received } => write!(
                formatter,
                "helper message contains {received} bytes; the maximum is {maximum}"
            ),
            Self::InvalidDocument(message) => {
                write!(
                    formatter,
                    "helper request is not a valid typed document: {message}"
                )
            }
            Self::VersionMismatch { expected, received } => write!(
                formatter,
                "helper protocol version {received} is unsupported; expected {expected}"
            ),
            Self::InvalidNonce => {
                formatter.write_str("helper nonce is not a valid version-four UUID")
            }
            Self::ReplayedNonce => formatter.write_str("helper nonce has already been used"),
            Self::WrongUser => {
                formatter.write_str("helper request does not belong to the current user")
            }
            Self::WrongParentIdentity => {
                formatter.write_str("helper parent Process identity does not match")
            }
            Self::WrongInteractiveSession => {
                formatter.write_str("helper caller is not in the same interactive Session")
            }
            Self::UnexpectedApplicationPath => {
                formatter.write_str("helper caller is not the exact Formation Lap sibling")
            }
            Self::UnverifiedReleaseIdentity => {
                formatter.write_str("helper caller release identity could not be verified")
            }
            Self::EmptyBatch => formatter.write_str("helper request does not contain an operation"),
            Self::BatchTooLarge { maximum, received } => write!(
                formatter,
                "helper request contains {received} operations; the maximum is {maximum}"
            ),
            Self::NonCanonicalPath(path) => {
                write!(formatter, "helper target is not canonical: {path}")
            }
            Self::InvalidExecutable(message) => {
                write!(formatter, "helper executable is not allowed: {message}")
            }
            Self::InvalidArguments(message) => {
                write!(formatter, "helper arguments are not allowed: {message}")
            }
            Self::WrongProcessIdentity(pid) => {
                write!(
                    formatter,
                    "helper Process identity does not match PID {pid}"
                )
            }
            Self::ProtectedProcess(pid) => {
                write!(formatter, "helper cannot control protected PID {pid}")
            }
        }
    }
}

impl Error for HelperProtocolError {}

#[derive(Default)]
pub struct ElevatedRequestValidator {
    consumed_nonces: BTreeSet<String>,
}

impl ElevatedRequestValidator {
    pub fn validate(
        &mut self,
        request: &ElevatedHelperRequest,
        context: &HelperValidationContext,
    ) -> Result<(), HelperProtocolError> {
        if request.protocol_version != ELEVATED_HELPER_PROTOCOL_VERSION {
            return Err(HelperProtocolError::VersionMismatch {
                expected: ELEVATED_HELPER_PROTOCOL_VERSION,
                received: request.protocol_version,
            });
        }
        validate_nonce(&request.nonce)?;
        if self.consumed_nonces.contains(&request.nonce) {
            return Err(HelperProtocolError::ReplayedNonce);
        }
        if request.current_user_id != context.current_user_id {
            return Err(HelperProtocolError::WrongUser);
        }
        if request.parent_identity != context.parent_identity {
            return Err(HelperProtocolError::WrongParentIdentity);
        }
        if !context.same_interactive_session {
            return Err(HelperProtocolError::WrongInteractiveSession);
        }
        if !context.expected_application_path {
            return Err(HelperProtocolError::UnexpectedApplicationPath);
        }
        if !context.release_identity_verified {
            return Err(HelperProtocolError::UnverifiedReleaseIdentity);
        }
        if request.operations.is_empty() {
            return Err(HelperProtocolError::EmptyBatch);
        }
        if request.operations.len() > MAX_ELEVATED_OPERATIONS {
            return Err(HelperProtocolError::BatchTooLarge {
                maximum: MAX_ELEVATED_OPERATIONS,
                received: request.operations.len(),
            });
        }

        for operation in &request.operations {
            validate_operation(operation, request, context)?;
        }
        self.consumed_nonces.insert(request.nonce.clone());
        Ok(())
    }
}

pub fn decode_helper_request(bytes: &[u8]) -> Result<ElevatedHelperRequest, HelperProtocolError> {
    if bytes.len() > MAX_HELPER_MESSAGE_BYTES {
        return Err(HelperProtocolError::MessageTooLarge {
            maximum: MAX_HELPER_MESSAGE_BYTES,
            received: bytes.len(),
        });
    }
    serde_json::from_slice(bytes)
        .map_err(|error| HelperProtocolError::InvalidDocument(error.to_string()))
}

pub fn encode_helper_message(message: &impl Serialize) -> Result<Vec<u8>, HelperProtocolError> {
    let bytes = serde_json::to_vec(message)
        .map_err(|error| HelperProtocolError::InvalidDocument(error.to_string()))?;
    if bytes.len() > MAX_HELPER_MESSAGE_BYTES {
        return Err(HelperProtocolError::MessageTooLarge {
            maximum: MAX_HELPER_MESSAGE_BYTES,
            received: bytes.len(),
        });
    }
    Ok(bytes)
}

fn validate_nonce(nonce: &str) -> Result<(), HelperProtocolError> {
    let nonce = Uuid::parse_str(nonce).map_err(|_| HelperProtocolError::InvalidNonce)?;
    if nonce.get_version_num() != 4 || nonce.is_nil() {
        return Err(HelperProtocolError::InvalidNonce);
    }
    Ok(())
}

fn validate_operation(
    operation: &ElevatedOperation,
    request: &ElevatedHelperRequest,
    context: &HelperValidationContext,
) -> Result<(), HelperProtocolError> {
    match operation {
        ElevatedOperation::Launch {
            executable_path,
            arguments,
            working_directory,
            monitored_process,
            monitored_executable_path,
            startup_timeout_seconds,
            ..
        } => {
            validate_executable(executable_path)?;
            validate_arguments(arguments)?;
            if let Some(working_directory) = working_directory {
                validate_canonical_directory(working_directory)?;
            }
            if let Some(monitored_process) = monitored_process {
                let monitored_path = Path::new(monitored_process);
                if monitored_process.trim().is_empty()
                    || !monitored_process.to_ascii_lowercase().ends_with(".exe")
                    || monitored_path.file_name().and_then(|name| name.to_str())
                        != Some(monitored_process)
                {
                    return Err(HelperProtocolError::InvalidExecutable(
                        "monitored Process must be an executable file name".to_owned(),
                    ));
                }
            }
            if let Some(monitored_executable_path) = monitored_executable_path {
                let canonical = validate_canonical_file(monitored_executable_path)?;
                if !canonical
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
                {
                    return Err(HelperProtocolError::InvalidExecutable(
                        "monitored executable path must identify an .exe file".to_owned(),
                    ));
                }
            }
            if !(1..=300).contains(startup_timeout_seconds) {
                return Err(HelperProtocolError::InvalidArguments(
                    "startup timeout must be between 1 and 300 seconds".to_owned(),
                ));
            }
            Ok(())
        }
        ElevatedOperation::GracefulStop {
            process_identity,
            strategy,
        } => {
            validate_controllable_process(process_identity, request, context)?;
            if let ShutdownStrategy::CustomStop {
                executable_path,
                arguments,
            } = strategy
            {
                validate_executable(executable_path)?;
                validate_arguments(arguments)?;
            }
            Ok(())
        }
        ElevatedOperation::ForceTerminate { process_identity } => {
            validate_controllable_process(process_identity, request, context)
        }
    }
}

fn validate_controllable_process(
    process_identity: &ProcessIdentity,
    request: &ElevatedHelperRequest,
    context: &HelperValidationContext,
) -> Result<(), HelperProtocolError> {
    if process_identity.pid == 0
        || process_identity.pid == context.helper_process_id
        || process_identity.pid == request.parent_identity.pid
    {
        return Err(HelperProtocolError::ProtectedProcess(process_identity.pid));
    }
    if !context
        .operation_process_identities
        .iter()
        .any(|observed| observed == process_identity)
    {
        return Err(HelperProtocolError::WrongProcessIdentity(
            process_identity.pid,
        ));
    }
    validate_canonical_file(&process_identity.canonical_executable_path)?;
    Ok(())
}

fn validate_executable(path: &str) -> Result<(), HelperProtocolError> {
    let canonical = validate_canonical_file(path)?;
    let executable_name = canonical
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            HelperProtocolError::InvalidExecutable("target has no executable name".to_owned())
        })?
        .to_ascii_lowercase();
    if !executable_name.ends_with(".exe") {
        return Err(HelperProtocolError::InvalidExecutable(
            "target is not an .exe file".to_owned(),
        ));
    }
    if matches!(
        executable_name.as_str(),
        "cmd.exe"
            | "powershell.exe"
            | "pwsh.exe"
            | "wscript.exe"
            | "cscript.exe"
            | "mshta.exe"
            | "rundll32.exe"
            | "regsvr32.exe"
    ) {
        return Err(HelperProtocolError::InvalidExecutable(
            "shell and script hosts are outside the helper protocol".to_owned(),
        ));
    }
    Ok(())
}

fn validate_canonical_file(path: &str) -> Result<PathBuf, HelperProtocolError> {
    let canonical = fs::canonicalize(path)
        .map_err(|_| HelperProtocolError::NonCanonicalPath(path.to_owned()))?;
    if !canonical.is_file() || canonical.to_string_lossy() != path {
        return Err(HelperProtocolError::NonCanonicalPath(path.to_owned()));
    }
    Ok(canonical)
}

fn validate_canonical_directory(path: &str) -> Result<(), HelperProtocolError> {
    let canonical = fs::canonicalize(path)
        .map_err(|_| HelperProtocolError::NonCanonicalPath(path.to_owned()))?;
    if !canonical.is_dir() || canonical.to_string_lossy() != path {
        return Err(HelperProtocolError::NonCanonicalPath(path.to_owned()));
    }
    Ok(())
}

fn validate_arguments(arguments: &[String]) -> Result<(), HelperProtocolError> {
    if arguments.len() > MAX_ELEVATED_ARGUMENTS {
        return Err(HelperProtocolError::InvalidArguments(format!(
            "operation contains {} arguments; the maximum is {MAX_ELEVATED_ARGUMENTS}",
            arguments.len()
        )));
    }
    let total_bytes = arguments.iter().map(String::len).sum::<usize>();
    if total_bytes > MAX_ELEVATED_ARGUMENT_BYTES {
        return Err(HelperProtocolError::InvalidArguments(format!(
            "operation arguments contain {total_bytes} bytes; the maximum is {MAX_ELEVATED_ARGUMENT_BYTES}"
        )));
    }
    if arguments
        .iter()
        .any(|argument| argument.contains(['\0', '\r', '\n']))
    {
        return Err(HelperProtocolError::InvalidArguments(
            "arguments cannot contain NUL or line separators".to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn canonical_executable_path(
    path: impl AsRef<Path>,
) -> Result<String, HelperProtocolError> {
    let canonical = fs::canonicalize(path.as_ref()).map_err(|_| {
        HelperProtocolError::NonCanonicalPath(path.as_ref().to_string_lossy().into_owned())
    })?;
    let canonical = canonical.to_string_lossy().into_owned();
    validate_executable(&canonical)?;
    Ok(canonical)
}
