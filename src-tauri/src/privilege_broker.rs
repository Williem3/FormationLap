use crate::{
    ELEVATED_HELPER_PROTOCOL_VERSION, ElevatedHelperRequest, ElevatedHelperResponse,
    ElevatedOperation, ElevatedOperationResult, ElevatedOwnershipAcknowledgement,
    ElevatedOwnershipOffer, ProcessIdentity,
};
use std::{
    collections::{BTreeSet, VecDeque},
    error::Error,
    fmt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrivilegeBrokerError {
    message: String,
}

impl PrivilegeBrokerError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PrivilegeBrokerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for PrivilegeBrokerError {}

type LaunchAcknowledgement<'a> =
    &'a mut dyn FnMut(usize, &ProcessIdentity) -> Result<(), PrivilegeBrokerError>;

/// Executes one fully typed privileged batch and returns one result per operation.
pub trait PrivilegeBroker: Send {
    fn execute(
        &mut self,
        operations: &[ElevatedOperation],
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError>;

    fn execute_launch_batch(
        &mut self,
        operations: &[ElevatedOperation],
        acknowledge: &mut dyn FnMut(usize, &ProcessIdentity) -> Result<(), PrivilegeBrokerError>,
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError>;
}

#[derive(Default)]
struct DevelopmentBrokerState {
    batches: Vec<Vec<ElevatedOperation>>,
    responses: VecDeque<Result<ElevatedHelperResponse, PrivilegeBrokerError>>,
}

/// In-process development adapter used to prove batching without triggering UAC.
#[derive(Clone, Default)]
pub struct DevelopmentPrivilegeBroker {
    state: Arc<Mutex<DevelopmentBrokerState>>,
}

impl DevelopmentPrivilegeBroker {
    pub fn queue_response(&self, response: Result<ElevatedHelperResponse, PrivilegeBrokerError>) {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .responses
            .push_back(response);
    }

    pub fn recorded_batches(&self) -> Vec<Vec<ElevatedOperation>> {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .batches
            .clone()
    }
}

impl PrivilegeBroker for DevelopmentPrivilegeBroker {
    fn execute(
        &mut self,
        operations: &[ElevatedOperation],
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError> {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.batches.push(operations.to_vec());
        state.responses.pop_front().unwrap_or_else(|| {
            Err(PrivilegeBrokerError::new(
                "development PrivilegeBroker does not have a queued response",
            ))
        })
    }

    fn execute_launch_batch(
        &mut self,
        operations: &[ElevatedOperation],
        acknowledge: &mut dyn FnMut(usize, &ProcessIdentity) -> Result<(), PrivilegeBrokerError>,
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError> {
        if operations
            .iter()
            .any(|operation| !matches!(operation, ElevatedOperation::Launch { .. }))
        {
            return Err(PrivilegeBrokerError::new(
                "ownership acknowledgement accepts only launch operations",
            ));
        }
        let response = self.execute(operations)?;
        for (operation_index, result) in response.results.iter().enumerate() {
            if let ElevatedOperationResult::Launched { process_identity } = result {
                acknowledge(operation_index, process_identity)?;
            }
        }
        Ok(response)
    }
}

/// Production PrivilegeBroker that invokes the bundled helper once through UAC.
pub struct WindowsPrivilegeBroker {
    helper_path: PathBuf,
}

impl WindowsPrivilegeBroker {
    pub fn new() -> Result<Self, PrivilegeBrokerError> {
        let executable = std::env::current_exe().map_err(|error| {
            broker_error("Formation Lap executable could not be located", error)
        })?;
        let helper_path = executable
            .parent()
            .ok_or_else(|| {
                PrivilegeBrokerError::new("Formation Lap executable has no installation directory")
            })?
            .join("formation-lap-elevated-helper.exe");
        Ok(Self { helper_path })
    }

    pub fn from_helper_path(path: impl AsRef<Path>) -> Result<Self, PrivilegeBrokerError> {
        let helper_path = crate::privilege_protocol::canonical_executable_path(path.as_ref())
            .map_err(|error| {
                PrivilegeBrokerError::new(format!("elevated helper is unavailable: {error}"))
            })?;
        let helper_path = PathBuf::from(helper_path);
        let helper_name = helper_path.file_name().and_then(|name| name.to_str());
        if !helper_name
            .is_some_and(|name| name.eq_ignore_ascii_case("formation-lap-elevated-helper.exe"))
        {
            return Err(PrivilegeBrokerError::new(
                "elevated helper has an unexpected executable name",
            ));
        }
        Ok(Self { helper_path })
    }

    #[cfg(feature = "process-fixtures")]
    #[doc(hidden)]
    pub fn execute_without_elevation_for_test(
        &mut self,
        operations: &[ElevatedOperation],
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError> {
        let helper_path = crate::privilege_protocol::canonical_executable_path(&self.helper_path)
            .map_err(|error| {
            PrivilegeBrokerError::new(format!("elevated helper is unavailable: {error}"))
        })?;
        platform::execute_without_uac_for_test(Path::new(&helper_path), operations)
    }

    #[cfg(feature = "process-fixtures")]
    #[doc(hidden)]
    pub fn execute_launch_batch_without_elevation_for_test(
        &mut self,
        operations: &[ElevatedOperation],
        acknowledge: &mut dyn FnMut(usize, &ProcessIdentity) -> Result<(), PrivilegeBrokerError>,
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError> {
        let helper_path = crate::privilege_protocol::canonical_executable_path(&self.helper_path)
            .map_err(|error| {
            PrivilegeBrokerError::new(format!("elevated helper is unavailable: {error}"))
        })?;
        platform::execute_launch_batch_without_uac_for_test(
            Path::new(&helper_path),
            operations,
            acknowledge,
        )
    }
}

impl PrivilegeBroker for WindowsPrivilegeBroker {
    fn execute(
        &mut self,
        operations: &[ElevatedOperation],
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError> {
        let helper_path = crate::privilege_protocol::canonical_executable_path(&self.helper_path)
            .map_err(|error| {
            PrivilegeBrokerError::new(format!("elevated helper is unavailable: {error}"))
        })?;
        platform::execute_with_uac(Path::new(&helper_path), operations)
    }

    fn execute_launch_batch(
        &mut self,
        operations: &[ElevatedOperation],
        acknowledge: &mut dyn FnMut(usize, &ProcessIdentity) -> Result<(), PrivilegeBrokerError>,
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError> {
        let helper_path = crate::privilege_protocol::canonical_executable_path(&self.helper_path)
            .map_err(|error| {
            PrivilegeBrokerError::new(format!("elevated helper is unavailable: {error}"))
        })?;
        platform::execute_launch_batch_with_uac(Path::new(&helper_path), operations, acknowledge)
    }
}

fn broker_error(context: &str, error: impl fmt::Display) -> PrivilegeBrokerError {
    PrivilegeBrokerError::new(format!("{context}: {error}"))
}

/// Entrypoint used only by the separately built one-shot helper binary.
pub fn run_elevated_helper(pipe_name: &str) -> Result<(), PrivilegeBrokerError> {
    platform::run_helper(pipe_name, false)
}

#[cfg(feature = "process-fixtures")]
#[doc(hidden)]
pub fn run_elevated_helper_for_test(pipe_name: &str) -> Result<(), PrivilegeBrokerError> {
    platform::run_helper(pipe_name, true)
}

#[cfg(not(windows))]
mod platform {
    use super::*;

    pub(super) fn execute_with_uac(
        _helper_path: &Path,
        _operations: &[ElevatedOperation],
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError> {
        Err(PrivilegeBrokerError::new(
            "privileged operations require Windows",
        ))
    }

    pub(super) fn execute_launch_batch_with_uac(
        _helper_path: &Path,
        _operations: &[ElevatedOperation],
        _acknowledge: &mut dyn FnMut(usize, &ProcessIdentity) -> Result<(), PrivilegeBrokerError>,
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError> {
        Err(PrivilegeBrokerError::new(
            "privileged operations require Windows",
        ))
    }

    pub(super) fn run_helper(
        _pipe_name: &str,
        _allow_test_caller: bool,
    ) -> Result<(), PrivilegeBrokerError> {
        Err(PrivilegeBrokerError::new(
            "the elevated helper requires Windows",
        ))
    }

    #[cfg(feature = "process-fixtures")]
    pub(super) fn execute_without_uac_for_test(
        _helper_path: &Path,
        _operations: &[ElevatedOperation],
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError> {
        Err(PrivilegeBrokerError::new(
            "the helper process fixture requires Windows",
        ))
    }

    #[cfg(feature = "process-fixtures")]
    pub(super) fn execute_launch_batch_without_uac_for_test(
        _helper_path: &Path,
        _operations: &[ElevatedOperation],
        _acknowledge: &mut dyn FnMut(usize, &ProcessIdentity) -> Result<(), PrivilegeBrokerError>,
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError> {
        Err(PrivilegeBrokerError::new(
            "the helper process fixture requires Windows",
        ))
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use crate::{
        ElevatedRequestValidator, GracefulStopResult, HelperValidationContext, LaunchRecipe,
        LaunchSource, ProcessRuntime, ShutdownStrategy, WindowsProcessRuntime,
        decode_helper_request, encode_helper_message,
    };
    use std::{
        ffi::{OsStr, c_void},
        fs::File,
        io::{self, Read, Write},
        mem::size_of,
        os::windows::{
            ffi::OsStrExt,
            io::{AsRawHandle, FromRawHandle},
        },
        ptr, thread,
        time::{Duration, Instant},
    };
    #[cfg(feature = "process-fixtures")]
    use std::{
        os::windows::{io::IntoRawHandle, process::CommandExt},
        process::Command,
    };
    use uuid::Uuid;
    use windows_sys::Win32::{
        Foundation::{
            CloseHandle, ERROR_PIPE_CONNECTED, ERROR_PIPE_LISTENING, GENERIC_READ, GENERIC_WRITE,
            HANDLE, INVALID_HANDLE_VALUE, LocalFree, WAIT_OBJECT_0, WAIT_TIMEOUT,
        },
        Security::{
            Authorization::{
                ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
                SDDL_REVISION_1,
            },
            GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY,
            TOKEN_USER, TokenUser,
        },
        Storage::FileSystem::{
            CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
        },
        System::{
            Pipes::{
                ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeClientProcessId,
                GetNamedPipeServerProcessId, PIPE_NOWAIT, PIPE_READMODE_BYTE,
                PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_WAIT, SetNamedPipeHandleState,
            },
            RemoteDesktop::ProcessIdToSessionId,
            Threading::{
                GetCurrentProcess, GetCurrentProcessId, GetProcessId, OpenProcess,
                OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION, TerminateProcess,
                WaitForSingleObject,
            },
        },
        UI::{
            Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW},
            WindowsAndMessaging::SW_HIDE,
        },
    };

    const PIPE_PREFIX: &str = r"\\.\pipe\FormationLap-";
    const HELPER_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
    const HELPER_EXIT_TIMEOUT_MILLISECONDS: u32 = 5_000;

    struct OwnedHandle(HANDLE);

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
    }

    struct HelperProcess(OwnedHandle);

    impl HelperProcess {
        fn pid(&self) -> Result<u32, PrivilegeBrokerError> {
            let pid = unsafe { GetProcessId(self.0.0) };
            if pid == 0 {
                return Err(last_error("elevated helper PID could not be read"));
            }
            Ok(pid)
        }

        fn wait_for_exit(&self) -> Result<(), PrivilegeBrokerError> {
            match unsafe { WaitForSingleObject(self.0.0, HELPER_EXIT_TIMEOUT_MILLISECONDS) } {
                WAIT_OBJECT_0 => Ok(()),
                WAIT_TIMEOUT => Err(PrivilegeBrokerError::new(
                    "elevated helper did not exit after its one-shot request",
                )),
                _ => Err(last_error("elevated helper exit could not be observed")),
            }
        }
    }

    impl Drop for HelperProcess {
        fn drop(&mut self) {
            if unsafe { WaitForSingleObject(self.0.0, 0) } == WAIT_TIMEOUT {
                unsafe {
                    TerminateProcess(self.0.0, 1);
                    WaitForSingleObject(self.0.0, HELPER_EXIT_TIMEOUT_MILLISECONDS);
                }
            }
        }
    }

    struct LocalAllocation(*mut c_void);

    impl Drop for LocalAllocation {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    LocalFree(self.0);
                }
            }
        }
    }

    struct PipeConnection(File);

    impl PipeConnection {
        fn send<T: serde::Serialize>(&mut self, message: &T) -> Result<(), PrivilegeBrokerError> {
            let bytes = encode_helper_message(message)
                .map_err(|error| PrivilegeBrokerError::new(error.to_string()))?;
            let length = u32::try_from(bytes.len())
                .map_err(|_| PrivilegeBrokerError::new("helper message length overflowed"))?;
            self.0
                .write_all(&length.to_le_bytes())
                .and_then(|()| self.0.write_all(&bytes))
                .and_then(|()| self.0.flush())
                .map_err(|error| broker_error("helper message could not be sent", error))
        }

        fn receive(&mut self) -> Result<Vec<u8>, PrivilegeBrokerError> {
            let mut length = [0_u8; 4];
            self.0
                .read_exact(&mut length)
                .map_err(|error| broker_error("helper response length could not be read", error))?;
            let length = usize::try_from(u32::from_le_bytes(length))
                .map_err(|_| PrivilegeBrokerError::new("helper response length is invalid"))?;
            if length > crate::MAX_HELPER_MESSAGE_BYTES {
                return Err(PrivilegeBrokerError::new(format!(
                    "helper response contains {length} bytes; the maximum is {}",
                    crate::MAX_HELPER_MESSAGE_BYTES
                )));
            }
            let mut bytes = vec![0_u8; length];
            self.0
                .read_exact(&mut bytes)
                .map_err(|error| broker_error("helper response could not be read", error))?;
            Ok(bytes)
        }
    }

    struct PipeServer {
        connection: PipeConnection,
    }

    impl PipeServer {
        fn create(pipe_name: &str, current_user_id: &str) -> Result<Self, PrivilegeBrokerError> {
            let security_descriptor = format!("D:P(A;;GA;;;{current_user_id})");
            let wide_security_descriptor = wide(&security_descriptor);
            let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
            if unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    wide_security_descriptor.as_ptr(),
                    SDDL_REVISION_1,
                    &mut descriptor,
                    ptr::null_mut(),
                )
            } == 0
            {
                return Err(last_error(
                    "current-user helper pipe security could not be created",
                ));
            }
            let _descriptor = LocalAllocation(descriptor);
            let security_attributes = SECURITY_ATTRIBUTES {
                nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>())
                    .map_err(|_| PrivilegeBrokerError::new("security attribute size overflowed"))?,
                lpSecurityDescriptor: descriptor,
                bInheritHandle: 0,
            };
            let wide_pipe_name = wide(pipe_name);
            let handle = unsafe {
                CreateNamedPipeW(
                    wide_pipe_name.as_ptr(),
                    PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS,
                    1,
                    crate::MAX_HELPER_MESSAGE_BYTES as u32,
                    crate::MAX_HELPER_MESSAGE_BYTES as u32,
                    30_000,
                    &security_attributes,
                )
            };
            if handle == INVALID_HANDLE_VALUE {
                return Err(last_error("current-user helper pipe could not be created"));
            }
            let file = unsafe { File::from_raw_handle(handle.cast()) };
            Ok(Self {
                connection: PipeConnection(file),
            })
        }

        fn connect(&mut self) -> Result<(), PrivilegeBrokerError> {
            let handle = self.connection.0.as_raw_handle().cast();
            let deadline = Instant::now() + HELPER_CONNECT_TIMEOUT;
            loop {
                if unsafe { ConnectNamedPipe(handle, ptr::null_mut()) } != 0 {
                    break;
                }
                let error = io::Error::last_os_error();
                match error.raw_os_error().map(|code| code as u32) {
                    Some(ERROR_PIPE_CONNECTED) => break,
                    Some(ERROR_PIPE_LISTENING) if Instant::now() < deadline => {
                        thread::sleep(Duration::from_millis(25));
                    }
                    _ => return Err(broker_error("elevated helper could not connect", error)),
                }
            }
            let wait_mode = PIPE_WAIT | PIPE_READMODE_BYTE;
            if unsafe { SetNamedPipeHandleState(handle, &wait_mode, ptr::null(), ptr::null()) } == 0
            {
                return Err(last_error("helper pipe could not enter blocking mode"));
            }
            Ok(())
        }

        fn client_pid(&self) -> Result<u32, PrivilegeBrokerError> {
            let mut pid = 0;
            if unsafe {
                GetNamedPipeClientProcessId(self.connection.0.as_raw_handle().cast(), &mut pid)
            } == 0
            {
                return Err(last_error("helper pipe client identity could not be read"));
            }
            Ok(pid)
        }
    }

    pub(super) fn execute_with_uac(
        helper_path: &Path,
        operations: &[ElevatedOperation],
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError> {
        if operations
            .iter()
            .any(|operation| matches!(operation, ElevatedOperation::Launch { .. }))
        {
            return Err(PrivilegeBrokerError::new(
                "elevated launches require an ownership acknowledgement",
            ));
        }
        execute_request(helper_path, operations, launch_helper, false, None)
    }

    pub(super) fn execute_launch_batch_with_uac(
        helper_path: &Path,
        operations: &[ElevatedOperation],
        acknowledge: &mut dyn FnMut(usize, &ProcessIdentity) -> Result<(), PrivilegeBrokerError>,
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError> {
        if operations
            .iter()
            .any(|operation| !matches!(operation, ElevatedOperation::Launch { .. }))
        {
            return Err(PrivilegeBrokerError::new(
                "ownership acknowledgement accepts only launch operations",
            ));
        }
        execute_request(
            helper_path,
            operations,
            launch_helper,
            false,
            Some(acknowledge),
        )
    }

    #[cfg(feature = "process-fixtures")]
    pub(super) fn execute_without_uac_for_test(
        helper_path: &Path,
        operations: &[ElevatedOperation],
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError> {
        let mut acknowledge = |_operation_index: usize, _identity: &ProcessIdentity| Ok(());
        let acknowledge = operations
            .iter()
            .any(|operation| matches!(operation, ElevatedOperation::Launch { .. }))
            .then_some(
                &mut acknowledge
                    as &mut dyn FnMut(usize, &ProcessIdentity) -> Result<(), PrivilegeBrokerError>,
            );
        execute_request(
            helper_path,
            operations,
            launch_helper_without_uac,
            true,
            acknowledge,
        )
    }

    #[cfg(feature = "process-fixtures")]
    pub(super) fn execute_launch_batch_without_uac_for_test(
        helper_path: &Path,
        operations: &[ElevatedOperation],
        acknowledge: &mut dyn FnMut(usize, &ProcessIdentity) -> Result<(), PrivilegeBrokerError>,
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError> {
        if operations
            .iter()
            .any(|operation| !matches!(operation, ElevatedOperation::Launch { .. }))
        {
            return Err(PrivilegeBrokerError::new(
                "ownership acknowledgement accepts only launch operations",
            ));
        }
        execute_request(
            helper_path,
            operations,
            launch_helper_without_uac,
            true,
            Some(acknowledge),
        )
    }

    fn execute_request(
        helper_path: &Path,
        operations: &[ElevatedOperation],
        launch: fn(&Path, &str) -> Result<HelperProcess, PrivilegeBrokerError>,
        allow_test_caller: bool,
        mut acknowledge: Option<LaunchAcknowledgement<'_>>,
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError> {
        if operations.is_empty() {
            return Err(PrivilegeBrokerError::new(
                "an elevated batch must contain at least one operation",
            ));
        }
        let _release_identity_guard = if !allow_test_caller {
            let main_executable = std::env::current_exe().map_err(|error| {
                broker_error("Formation Lap executable could not be located", error)
            })?;
            Some(
                crate::release_identity::verify_runtime_release_identity(
                    &main_executable,
                    helper_path,
                )
                .map_err(|error| {
                    PrivilegeBrokerError::new(format!(
                        "elevated helper release identity was rejected: {error}"
                    ))
                })?,
            )
        } else {
            None
        };
        let current_user_id = current_user_id()?;
        let nonce = Uuid::new_v4().to_string();
        let pipe_name = format!("{PIPE_PREFIX}{nonce}");
        let mut pipe = PipeServer::create(&pipe_name, &current_user_id)?;
        let parent_identity =
            crate::process_runtime::process_identity_for_pid(unsafe { GetCurrentProcessId() })
                .map_err(|error| broker_error("parent Process identity is unavailable", error))?;
        let request = ElevatedHelperRequest {
            protocol_version: ELEVATED_HELPER_PROTOCOL_VERSION,
            parent_identity,
            nonce: nonce.clone(),
            current_user_id,
            operations: operations.to_vec(),
        };

        let helper = launch(helper_path, &pipe_name)?;
        pipe.connect()?;
        if pipe.client_pid()? != helper.pid()? {
            return Err(PrivilegeBrokerError::new(
                "helper pipe client is not the UAC-launched helper Process",
            ));
        }
        pipe.connection.send(&request)?;
        let mut acknowledged_launches = BTreeSet::new();
        let response = loop {
            let response_bytes = pipe.connection.receive()?;
            if let Ok(offer) = serde_json::from_slice::<ElevatedOwnershipOffer>(&response_bytes) {
                let offer_is_valid = offer.protocol_version == ELEVATED_HELPER_PROTOCOL_VERSION
                    && offer.nonce == nonce
                    && matches!(
                        operations.get(offer.operation_index),
                        Some(ElevatedOperation::Launch { .. })
                    )
                    && acknowledged_launches.insert(offer.operation_index);
                if !offer_is_valid {
                    return abandon_pending_launch(
                        pipe,
                        &helper,
                        PrivilegeBrokerError::new(
                            "helper ownership offer does not match the request",
                        ),
                    );
                }
                let Some(acknowledge) = acknowledge.as_deref_mut() else {
                    return abandon_pending_launch(
                        pipe,
                        &helper,
                        PrivilegeBrokerError::new(
                            "helper requested ownership acknowledgement for an untracked launch",
                        ),
                    );
                };
                if let Err(error) = acknowledge(offer.operation_index, &offer.process_identity) {
                    return abandon_pending_launch(pipe, &helper, error);
                }
                if let Err(error) = pipe.connection.send(&ElevatedOwnershipAcknowledgement {
                    protocol_version: ELEVATED_HELPER_PROTOCOL_VERSION,
                    nonce: nonce.clone(),
                    operation_index: offer.operation_index,
                    process_identity: offer.process_identity,
                }) {
                    return abandon_pending_launch(pipe, &helper, error);
                }
                continue;
            }
            break serde_json::from_slice::<ElevatedHelperResponse>(&response_bytes)
                .map_err(|error| broker_error("helper response is invalid", error))?;
        };
        helper.wait_for_exit()?;

        if response.protocol_version != ELEVATED_HELPER_PROTOCOL_VERSION {
            return Err(PrivilegeBrokerError::new(
                "helper response protocol version does not match",
            ));
        }
        if response.nonce != nonce {
            return Err(PrivilegeBrokerError::new(
                "helper response nonce does not match",
            ));
        }
        if !response.accepted {
            return Err(PrivilegeBrokerError::new(response.error.unwrap_or_else(
                || "elevated helper rejected the request".to_owned(),
            )));
        }
        if response.results.len() != operations.len() {
            return Err(PrivilegeBrokerError::new(
                "helper response does not contain one result per operation",
            ));
        }
        if response.results.iter().enumerate().any(|(index, result)| {
            matches!(result, ElevatedOperationResult::Launched { .. })
                && !acknowledged_launches.contains(&index)
        }) {
            return Err(PrivilegeBrokerError::new(
                "helper returned an unacknowledged launched Process",
            ));
        }
        Ok(response)
    }

    fn abandon_pending_launch(
        pipe: PipeServer,
        helper: &HelperProcess,
        error: PrivilegeBrokerError,
    ) -> Result<ElevatedHelperResponse, PrivilegeBrokerError> {
        drop(pipe);
        let _ = helper.wait_for_exit();
        Err(error)
    }

    pub(super) fn run_helper(
        pipe_name: &str,
        allow_test_caller: bool,
    ) -> Result<(), PrivilegeBrokerError> {
        let nonce = validate_pipe_name(pipe_name)?;
        let mut pipe = open_pipe_client(pipe_name)?;
        let server_pid = server_pid(&pipe)?;
        let request_bytes = pipe.receive()?;
        let request = match decode_helper_request(&request_bytes) {
            Ok(request) => request,
            Err(error) => {
                send_rejection(&mut pipe, &nonce, error.to_string())?;
                return Err(PrivilegeBrokerError::new(error.to_string()));
            }
        };
        if request.nonce != nonce {
            let message = "helper pipe nonce does not match the request".to_owned();
            send_rejection(&mut pipe, &nonce, message.clone())?;
            return Err(PrivilegeBrokerError::new(message));
        }

        let parent_identity = crate::process_runtime::process_identity_for_pid(server_pid)
            .map_err(|error| {
                PrivilegeBrokerError::new(format!(
                    "helper pipe server identity is unavailable: {error}"
                ))
            })?;
        let helper_process_id = unsafe { GetCurrentProcessId() };
        let helper_executable = std::env::current_exe()
            .map_err(|error| broker_error("elevated helper path is unavailable", error))?;
        let server_user_id = process_user_id(server_pid)?;
        let helper_user_id = current_user_id()?;
        let same_user = server_user_id == helper_user_id;
        let same_interactive_session = matches!(
            (
                process_session_id(server_pid),
                process_session_id(helper_process_id)
            ),
            (Ok(server), Ok(helper)) if server == helper
        );
        let (expected_application_path, release_identity_verified, _release_identity_guard) =
            if allow_test_caller {
                (true, true, None)
            } else {
                let main_executable =
                    Path::new(&parent_identity.canonical_executable_path).to_path_buf();
                let expected = crate::release_identity::validate_expected_application_pair(
                    &main_executable,
                    &helper_executable,
                )
                .is_ok();
                let release_identity = expected
                    .then(|| {
                        crate::release_identity::verify_runtime_release_identity(
                            &main_executable,
                            &helper_executable,
                        )
                    })
                    .transpose()
                    .ok()
                    .flatten();
                (expected, release_identity.is_some(), release_identity)
            };
        let operation_process_identities = request
            .operations
            .iter()
            .filter_map(operation_process_identity)
            .filter_map(|identity| {
                crate::process_runtime::process_identity_for_pid(identity.pid).ok()
            })
            .collect();
        let context = HelperValidationContext {
            current_user_id: if same_user {
                server_user_id
            } else {
                helper_user_id
            },
            parent_identity,
            helper_process_id,
            operation_process_identities,
            same_interactive_session,
            expected_application_path,
            release_identity_verified,
        };
        if let Err(error) = ElevatedRequestValidator::default().validate(&request, &context) {
            send_rejection(&mut pipe, &nonce, error.to_string())?;
            return Err(PrivilegeBrokerError::new(error.to_string()));
        }

        let mut results = Vec::with_capacity(request.operations.len());
        for (operation_index, operation) in request.operations.iter().enumerate() {
            let result = execute_operation(operation);
            if let ElevatedOperationResult::Launched { process_identity } = &result {
                let ownership = ElevatedOwnershipOffer {
                    protocol_version: ELEVATED_HELPER_PROTOCOL_VERSION,
                    nonce: nonce.clone(),
                    operation_index,
                    process_identity: process_identity.clone(),
                };
                if let Err(error) = pipe.send(&ownership) {
                    compensate_unacknowledged_launch(process_identity);
                    return Err(error);
                }
                let acknowledgement = pipe
                    .receive()
                    .and_then(|bytes| {
                        serde_json::from_slice::<ElevatedOwnershipAcknowledgement>(&bytes).map_err(
                            |error| broker_error("ownership acknowledgement is invalid", error),
                        )
                    })
                    .and_then(|acknowledgement| {
                        if acknowledgement.protocol_version == ELEVATED_HELPER_PROTOCOL_VERSION
                            && acknowledgement.nonce == nonce
                            && acknowledgement.operation_index == operation_index
                            && acknowledgement.process_identity == *process_identity
                        {
                            Ok(())
                        } else {
                            Err(PrivilegeBrokerError::new(
                                "ownership acknowledgement does not match the launched Process",
                            ))
                        }
                    });
                if let Err(error) = acknowledgement {
                    compensate_unacknowledged_launch(process_identity);
                    let _ = send_rejection(&mut pipe, &nonce, error.to_string());
                    return Err(error);
                }
            }
            results.push(result);
        }
        pipe.send(&ElevatedHelperResponse {
            protocol_version: ELEVATED_HELPER_PROTOCOL_VERSION,
            nonce,
            accepted: true,
            error: None,
            results,
        })
    }

    fn compensate_unacknowledged_launch(identity: &ProcessIdentity) {
        let _ = WindowsProcessRuntime::new().force_stop(identity);
    }

    fn operation_process_identity(
        operation: &ElevatedOperation,
    ) -> Option<&crate::ProcessIdentity> {
        match operation {
            ElevatedOperation::GracefulStop {
                process_identity, ..
            }
            | ElevatedOperation::ForceTerminate { process_identity } => Some(process_identity),
            ElevatedOperation::Launch { .. } => None,
        }
    }

    fn execute_operation(operation: &ElevatedOperation) -> ElevatedOperationResult {
        let result = match operation {
            ElevatedOperation::Launch {
                executable_path,
                executable_sha256,
                arguments,
                working_directory,
                monitored_process,
                monitored_executable_path,
                console_visibility,
                startup_timeout_seconds,
            } => crate::privilege_protocol::VerifiedExecutableTarget::open(
                executable_path,
                executable_sha256,
            )
            .map_err(|error| crate::ProcessRuntimeError::new(error.to_string()))
            .and_then(|verified| {
                crate::process_runtime::launch_without_output_capture(&LaunchRecipe {
                    source: LaunchSource::DirectExecutable {
                        executable_path: verified.canonical_path().to_owned(),
                    },
                    arguments: arguments.clone(),
                    working_directory: working_directory.clone(),
                    monitored_process: monitored_process.clone(),
                    monitored_executable_path: monitored_executable_path.clone(),
                    console_visibility: console_visibility.clone(),
                    elevated: false,
                    startup_timeout_seconds: *startup_timeout_seconds,
                    post_start_delay_milliseconds: 0,
                    shutdown_strategy: ShutdownStrategy::CloseWindows,
                })
            })
            .map(|process_identity| ElevatedOperationResult::Launched { process_identity }),
            ElevatedOperation::GracefulStop {
                process_identity,
                strategy,
                custom_stop_executable_sha256,
            } => (|| -> Result<ElevatedOperationResult, crate::ProcessRuntimeError> {
                let verified = match strategy {
                    ShutdownStrategy::CustomStop {
                        executable_path, ..
                    } => {
                        let expected =
                            custom_stop_executable_sha256.as_deref().ok_or_else(|| {
                                crate::ProcessRuntimeError::new(
                                    "custom-stop target is missing its approved executable hash",
                                )
                            })?;
                        Some(
                            crate::privilege_protocol::VerifiedExecutableTarget::open(
                                executable_path,
                                expected,
                            )
                            .map_err(|error| crate::ProcessRuntimeError::new(error.to_string()))?,
                        )
                    }
                    _ => None,
                };
                let strategy = match (strategy, &verified) {
                    (ShutdownStrategy::CustomStop { arguments, .. }, Some(verified)) => {
                        ShutdownStrategy::CustomStop {
                            executable_path: verified.canonical_path().to_owned(),
                            arguments: arguments.clone(),
                        }
                    }
                    _ => strategy.clone(),
                };
                let mut runtime = WindowsProcessRuntime::new();
                runtime
                    .request_graceful_stop(process_identity, &strategy)
                    .and_then(|result| {
                        let requested = result == GracefulStopResult::Requested;
                        let exited = requested
                            && runtime.wait_for_exit(process_identity, Duration::from_secs(5))?;
                        Ok(ElevatedOperationResult::GracefulStopRequested { requested, exited })
                    })
            })(),
            ElevatedOperation::ForceTerminate { process_identity } => WindowsProcessRuntime::new()
                .force_stop(process_identity)
                .map(|()| ElevatedOperationResult::ForceTerminated),
        };
        result.unwrap_or_else(|error| ElevatedOperationResult::Failed {
            message: error.to_string(),
        })
    }

    fn send_rejection(
        pipe: &mut PipeConnection,
        nonce: &str,
        message: String,
    ) -> Result<(), PrivilegeBrokerError> {
        pipe.send(&ElevatedHelperResponse {
            protocol_version: ELEVATED_HELPER_PROTOCOL_VERSION,
            nonce: nonce.to_owned(),
            accepted: false,
            error: Some(message),
            results: Vec::new(),
        })
    }

    fn launch_helper(
        helper_path: &Path,
        pipe_name: &str,
    ) -> Result<HelperProcess, PrivilegeBrokerError> {
        let verb = wide("runas");
        let helper = wide(helper_path.as_os_str());
        let parameters = wide(pipe_name);
        let mut execute = SHELLEXECUTEINFOW {
            cbSize: u32::try_from(size_of::<SHELLEXECUTEINFOW>())
                .map_err(|_| PrivilegeBrokerError::new("shell execute size overflowed"))?,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpVerb: verb.as_ptr(),
            lpFile: helper.as_ptr(),
            lpParameters: parameters.as_ptr(),
            nShow: SW_HIDE,
            ..SHELLEXECUTEINFOW::default()
        };
        if unsafe { ShellExecuteExW(&mut execute) } == 0 {
            return Err(last_error(
                "the elevated helper request was cancelled or could not start",
            ));
        }
        if execute.hProcess.is_null() {
            return Err(PrivilegeBrokerError::new(
                "the elevated helper did not return a Process handle",
            ));
        }
        Ok(HelperProcess(OwnedHandle(execute.hProcess)))
    }

    #[cfg(feature = "process-fixtures")]
    fn launch_helper_without_uac(
        helper_path: &Path,
        pipe_name: &str,
    ) -> Result<HelperProcess, PrivilegeBrokerError> {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let child = Command::new(helper_path)
            .arg(pipe_name)
            .arg("--allow-development-test-caller")
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|error| broker_error("helper process fixture could not start", error))?;
        let handle = child.into_raw_handle().cast();
        Ok(HelperProcess(OwnedHandle(handle)))
    }

    fn open_pipe_client(pipe_name: &str) -> Result<PipeConnection, PrivilegeBrokerError> {
        let pipe_name = wide(pipe_name);
        let handle = unsafe {
            CreateFileW(
                pipe_name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                ptr::null(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(last_error("current-user helper pipe could not be opened"));
        }
        let file = unsafe { File::from_raw_handle(handle.cast()) };
        Ok(PipeConnection(file))
    }

    fn server_pid(pipe: &PipeConnection) -> Result<u32, PrivilegeBrokerError> {
        let mut pid = 0;
        if unsafe { GetNamedPipeServerProcessId(pipe.0.as_raw_handle().cast(), &mut pid) } == 0 {
            return Err(last_error("helper pipe server identity could not be read"));
        }
        Ok(pid)
    }

    fn current_user_id() -> Result<String, PrivilegeBrokerError> {
        user_id_for_process(unsafe { GetCurrentProcess() })
    }

    fn process_user_id(process_id: u32) -> Result<String, PrivilegeBrokerError> {
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
        if process.is_null() {
            return Err(last_error("helper pipe server Process could not be opened"));
        }
        let process = OwnedHandle(process);
        user_id_for_process(process.0)
    }

    fn user_id_for_process(process: HANDLE) -> Result<String, PrivilegeBrokerError> {
        let mut token = ptr::null_mut();
        if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
            return Err(last_error("current-user token could not be opened"));
        }
        let token = OwnedHandle(token);
        let mut byte_count = 0;
        unsafe {
            GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut byte_count);
        }
        if byte_count == 0 {
            return Err(last_error("current-user token size could not be read"));
        }
        let word_count = usize::try_from(byte_count)
            .map_err(|_| PrivilegeBrokerError::new("current-user token size overflowed"))?
            .div_ceil(size_of::<usize>());
        let mut token_information = vec![0_usize; word_count];
        if unsafe {
            GetTokenInformation(
                token.0,
                TokenUser,
                token_information.as_mut_ptr().cast(),
                byte_count,
                &mut byte_count,
            )
        } == 0
        {
            return Err(last_error("current-user token could not be read"));
        }
        let token_user = unsafe { &*(token_information.as_ptr().cast::<TOKEN_USER>()) };
        let mut string_sid = ptr::null_mut();
        if unsafe { ConvertSidToStringSidW(token_user.User.Sid, &mut string_sid) } == 0 {
            return Err(last_error("current-user SID could not be formatted"));
        }
        let _string_sid = LocalAllocation(string_sid.cast());
        let mut length = 0;
        while unsafe { *string_sid.add(length) } != 0 {
            length += 1;
        }
        String::from_utf16(unsafe { std::slice::from_raw_parts(string_sid, length) })
            .map_err(|error| broker_error("current-user SID is invalid UTF-16", error))
    }

    fn process_session_id(process_id: u32) -> Result<u32, PrivilegeBrokerError> {
        let mut session_id = 0;
        if unsafe { ProcessIdToSessionId(process_id, &mut session_id) } == 0 {
            return Err(last_error("Process interactive Session could not be read"));
        }
        Ok(session_id)
    }

    fn validate_pipe_name(pipe_name: &str) -> Result<String, PrivilegeBrokerError> {
        let nonce = pipe_name
            .strip_prefix(PIPE_PREFIX)
            .ok_or_else(|| PrivilegeBrokerError::new("helper pipe name has an invalid prefix"))?;
        if nonce.contains(['\\', '/'])
            || Uuid::parse_str(nonce).is_err()
            || Uuid::parse_str(nonce).is_ok_and(|value| value.get_version_num() != 4)
        {
            return Err(PrivilegeBrokerError::new(
                "helper pipe name does not contain a version-four nonce",
            ));
        }
        Ok(nonce.to_owned())
    }

    fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
        value
            .as_ref()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn last_error(context: &str) -> PrivilegeBrokerError {
        broker_error(context, io::Error::last_os_error())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn current_user_pipe_authenticates_both_local_process_ids() {
            let pipe_name = format!("{PIPE_PREFIX}{}", Uuid::new_v4());
            let mut server = PipeServer::create(
                &pipe_name,
                &current_user_id().expect("the current-user SID should be available"),
            )
            .expect("the current-user-only pipe should be created");
            let client_pipe_name = pipe_name.clone();
            let client = thread::spawn(move || {
                let mut client = open_pipe_client(&client_pipe_name)
                    .expect("the same current user should open the pipe");
                assert_eq!(
                    server_pid(&client).expect("the server PID should be authenticated"),
                    std::process::id()
                );
                let request = client.receive().expect("the request should arrive");
                assert_eq!(
                    serde_json::from_slice::<String>(&request)
                        .expect("the request should be typed JSON"),
                    "ping"
                );
                client
                    .send(&"pong".to_owned())
                    .expect("the response should send");
            });

            server.connect().expect("the pipe client should connect");
            assert_eq!(
                server
                    .client_pid()
                    .expect("the client PID should be authenticated"),
                std::process::id()
            );
            server
                .connection
                .send(&"ping".to_owned())
                .expect("the request should send");
            assert_eq!(
                serde_json::from_slice::<String>(
                    &server
                        .connection
                        .receive()
                        .expect("the response should arrive")
                )
                .expect("the response should be typed JSON"),
                "pong"
            );
            client.join().expect("the pipe client should exit");
        }
    }
}
