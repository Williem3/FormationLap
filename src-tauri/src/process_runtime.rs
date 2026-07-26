use crate::{LaunchRecipe, ProcessIdentity, ProcessOutput, ShutdownStrategy};
#[cfg(windows)]
use std::collections::BTreeMap;
use std::{error::Error, fmt, time::Duration};

#[cfg(windows)]
pub(crate) fn running_executable_paths() -> Vec<std::path::PathBuf> {
    windows_adapter::running_executable_paths()
}

#[cfg(windows)]
pub(crate) fn process_identity_for_pid(pid: u32) -> Result<ProcessIdentity, ProcessRuntimeError> {
    windows_adapter::process_identity(pid)
}

#[cfg(not(windows))]
pub(crate) fn process_identity_for_pid(_pid: u32) -> Result<ProcessIdentity, ProcessRuntimeError> {
    Err(ProcessRuntimeError::new(
        "stable Process identity requires Windows",
    ))
}

#[cfg(windows)]
pub(crate) fn launch_without_output_capture(
    recipe: &LaunchRecipe,
) -> Result<ProcessIdentity, ProcessRuntimeError> {
    windows_adapter::launch_direct(recipe, false).map(|launched| launched.identity)
}

#[cfg(not(windows))]
pub(crate) fn launch_without_output_capture(
    _recipe: &LaunchRecipe,
) -> Result<ProcessIdentity, ProcessRuntimeError> {
    Err(ProcessRuntimeError::new(
        "elevated process launch requires Windows",
    ))
}

#[cfg(not(windows))]
pub(crate) fn running_executable_paths() -> Vec<std::path::PathBuf> {
    Vec::new()
}

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

/// Result of checking one exact stable Process identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProcessObservation {
    Running {
        responsiveness: ProcessResponsiveness,
    },
    Exited,
    Replaced {
        current_identity: ProcessIdentity,
    },
}

/// Whether a Process has a responsive top-level window.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProcessResponsiveness {
    NotApplicable,
    Responsive,
    NotResponsive,
}

/// Whether a graceful shutdown request was available for a Process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GracefulStopResult {
    Requested,
    Unavailable,
}

/// Observes and controls local processes without deciding Session policy.
pub trait ProcessRuntime: Send {
    fn matching_processes(
        &mut self,
        recipe: &LaunchRecipe,
    ) -> Result<Vec<ProcessIdentity>, ProcessRuntimeError>;

    fn launch(&mut self, recipe: &LaunchRecipe) -> Result<ProcessIdentity, ProcessRuntimeError>;

    fn observe(
        &mut self,
        identity: &ProcessIdentity,
    ) -> Result<ProcessObservation, ProcessRuntimeError>;

    fn request_graceful_stop(
        &mut self,
        identity: &ProcessIdentity,
        strategy: &ShutdownStrategy,
    ) -> Result<GracefulStopResult, ProcessRuntimeError>;

    fn wait_for_exit(
        &mut self,
        identity: &ProcessIdentity,
        timeout: Duration,
    ) -> Result<bool, ProcessRuntimeError>;

    fn force_stop(&mut self, identity: &ProcessIdentity) -> Result<(), ProcessRuntimeError>;

    fn read_output(
        &mut self,
        identity: &ProcessIdentity,
    ) -> Result<ProcessOutput, ProcessRuntimeError>;
}

/// Production ProcessRuntime adapter for Windows.
#[derive(Default)]
pub struct WindowsProcessRuntime {
    #[cfg(windows)]
    captured_output: BTreeMap<String, windows_adapter::CapturedOutput>,
    #[cfg(windows)]
    companions: BTreeMap<String, Vec<ProcessIdentity>>,
}

impl WindowsProcessRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(windows)]
    fn discover_companions(
        &mut self,
        identity: &ProcessIdentity,
    ) -> Result<(), ProcessRuntimeError> {
        let companions = self.companions.entry(identity_key(identity)).or_default();
        for companion in windows_adapter::same_executable_descendants(identity)? {
            if !companions
                .iter()
                .any(|known| identity_key(known) == identity_key(&companion))
            {
                companions.push(companion);
            }
        }
        Ok(())
    }

    #[cfg(windows)]
    fn tracked_identities(&self, identity: &ProcessIdentity) -> Vec<ProcessIdentity> {
        let mut identities = vec![identity.clone()];
        if let Some(companions) = self.companions.get(&identity_key(identity)) {
            identities.extend(companions.iter().cloned());
        }
        identities
    }

    #[cfg(windows)]
    fn forget_companions(&mut self, identity: &ProcessIdentity) {
        self.companions.remove(&identity_key(identity));
    }
}

impl ProcessRuntime for WindowsProcessRuntime {
    fn matching_processes(
        &mut self,
        recipe: &LaunchRecipe,
    ) -> Result<Vec<ProcessIdentity>, ProcessRuntimeError> {
        #[cfg(windows)]
        {
            windows_adapter::matching_processes(recipe)
        }
        #[cfg(not(windows))]
        {
            let _ = recipe;
            Err(ProcessRuntimeError::new(
                "local process observation requires Windows",
            ))
        }
    }

    fn launch(&mut self, recipe: &LaunchRecipe) -> Result<ProcessIdentity, ProcessRuntimeError> {
        #[cfg(windows)]
        {
            let launched = windows_adapter::launch(recipe)?;
            if let Some(output) = launched.captured_output {
                self.captured_output
                    .insert(identity_key(&launched.identity), output);
                if self.captured_output.len() > 32
                    && let Some(oldest_key) = self.captured_output.keys().next().cloned()
                {
                    self.captured_output.remove(&oldest_key);
                }
            }
            Ok(launched.identity)
        }
        #[cfg(not(windows))]
        {
            let _ = recipe;
            Err(ProcessRuntimeError::new(
                "local process launch requires Windows",
            ))
        }
    }

    fn observe(
        &mut self,
        identity: &ProcessIdentity,
    ) -> Result<ProcessObservation, ProcessRuntimeError> {
        #[cfg(windows)]
        {
            self.discover_companions(identity)?;
            let primary_observation = windows_adapter::observe(identity)?;
            if let ProcessObservation::Replaced { current_identity } = primary_observation {
                self.forget_companions(identity);
                return Ok(ProcessObservation::Replaced { current_identity });
            }

            let mut has_running_process = false;
            let mut has_responsive_window = false;
            let mut has_unresponsive_window = false;
            let mut retained_companions = Vec::new();

            for tracked in self.tracked_identities(identity) {
                let observation = if tracked == *identity {
                    primary_observation.clone()
                } else {
                    windows_adapter::observe(&tracked)?
                };
                match observation {
                    ProcessObservation::Running { responsiveness } => {
                        has_running_process = true;
                        match responsiveness {
                            ProcessResponsiveness::NotApplicable => {}
                            ProcessResponsiveness::Responsive => has_responsive_window = true,
                            ProcessResponsiveness::NotResponsive => has_unresponsive_window = true,
                        }
                        if tracked != *identity {
                            retained_companions.push(tracked);
                        }
                    }
                    ProcessObservation::Exited | ProcessObservation::Replaced { .. } => {}
                }
            }

            if retained_companions.is_empty() {
                self.forget_companions(identity);
            } else {
                self.companions
                    .insert(identity_key(identity), retained_companions);
            }

            if !has_running_process {
                return Ok(ProcessObservation::Exited);
            }
            let responsiveness = if has_unresponsive_window {
                ProcessResponsiveness::NotResponsive
            } else if has_responsive_window {
                ProcessResponsiveness::Responsive
            } else {
                ProcessResponsiveness::NotApplicable
            };
            Ok(ProcessObservation::Running { responsiveness })
        }
        #[cfg(not(windows))]
        {
            let _ = identity;
            Err(ProcessRuntimeError::new(
                "local process observation requires Windows",
            ))
        }
    }

    fn request_graceful_stop(
        &mut self,
        identity: &ProcessIdentity,
        strategy: &ShutdownStrategy,
    ) -> Result<GracefulStopResult, ProcessRuntimeError> {
        #[cfg(windows)]
        {
            self.discover_companions(identity)?;
            let targets = if matches!(strategy, ShutdownStrategy::CloseWindows) {
                self.tracked_identities(identity)
            } else {
                vec![identity.clone()]
            };
            let mut requested = false;
            for tracked in targets {
                if windows_adapter::request_graceful_stop(&tracked, strategy)?
                    == GracefulStopResult::Requested
                {
                    requested = true;
                }
            }
            Ok(if requested {
                GracefulStopResult::Requested
            } else {
                GracefulStopResult::Unavailable
            })
        }
        #[cfg(not(windows))]
        {
            let _ = (identity, strategy);
            Err(ProcessRuntimeError::new(
                "local process shutdown requires Windows",
            ))
        }
    }

    fn wait_for_exit(
        &mut self,
        identity: &ProcessIdentity,
        timeout: Duration,
    ) -> Result<bool, ProcessRuntimeError> {
        #[cfg(windows)]
        {
            self.discover_companions(identity)?;
            let deadline = std::time::Instant::now() + timeout;
            for tracked in self.tracked_identities(identity) {
                if !windows_adapter::wait_for_exit(
                    &tracked,
                    deadline.saturating_duration_since(std::time::Instant::now()),
                )? {
                    return Ok(false);
                }
            }
            self.forget_companions(identity);
            Ok(true)
        }
        #[cfg(not(windows))]
        {
            let _ = (identity, timeout);
            Err(ProcessRuntimeError::new(
                "local process shutdown requires Windows",
            ))
        }
    }

    fn force_stop(&mut self, identity: &ProcessIdentity) -> Result<(), ProcessRuntimeError> {
        #[cfg(windows)]
        {
            self.discover_companions(identity)?;
            for tracked in self.tracked_identities(identity) {
                windows_adapter::force_stop(&tracked)?;
            }
            self.forget_companions(identity);
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = identity;
            Err(ProcessRuntimeError::new(
                "local process shutdown requires Windows",
            ))
        }
    }

    fn read_output(
        &mut self,
        identity: &ProcessIdentity,
    ) -> Result<ProcessOutput, ProcessRuntimeError> {
        #[cfg(windows)]
        {
            Ok(self
                .captured_output
                .get(&identity_key(identity))
                .map(windows_adapter::CapturedOutput::read)
                .unwrap_or_default())
        }
        #[cfg(not(windows))]
        {
            let _ = identity;
            Err(ProcessRuntimeError::new(
                "local process output requires Windows",
            ))
        }
    }
}

#[cfg(windows)]
fn identity_key(identity: &ProcessIdentity) -> String {
    format!(
        "{}\u{0}{}\u{0}{}",
        identity.pid, identity.creation_time, identity.canonical_executable_path
    )
}

#[cfg(windows)]
mod windows_adapter {
    use super::{ProcessObservation, ProcessOutput, ProcessResponsiveness, ProcessRuntimeError};
    use crate::{ConsoleVisibility, LaunchRecipe, LaunchSource, ProcessIdentity};
    use std::{
        collections::BTreeSet,
        ffi::{OsStr, OsString},
        io::{self, Read},
        mem::size_of,
        os::windows::{
            ffi::{OsStrExt, OsStringExt},
            process::CommandExt,
        },
        path::{Path, PathBuf},
        process::{Command, Stdio},
        ptr,
        sync::{Arc, Mutex},
        thread,
        time::{Duration, Instant},
    };
    use windows_sys::Win32::{
        Foundation::{
            CloseHandle, FILETIME, HANDLE, HWND, INVALID_HANDLE_VALUE, LPARAM, SetLastError,
            WAIT_OBJECT_0, WAIT_TIMEOUT,
        },
        System::{
            Console::{CTRL_BREAK_EVENT, GenerateConsoleCtrlEvent},
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
                TH32CS_SNAPPROCESS,
            },
            Threading::{
                CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW, GetProcessTimes, OpenProcess,
                PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE, QueryFullProcessImageNameW,
                TerminateProcess, WaitForSingleObject,
            },
        },
        UI::{
            Shell::ShellExecuteW,
            WindowsAndMessaging::{
                EnumWindows, GetWindowThreadProcessId, IsHungAppWindow, IsWindowVisible,
                PostMessageW, SW_SHOWNORMAL, WM_CLOSE,
            },
        },
    };

    struct OwnedHandle(HANDLE);

    struct VerifiedProcessHandle {
        handle: OwnedHandle,
    }

    #[derive(Clone, Copy)]
    struct ProcessRights(u32);

    enum VerifiedProcessError {
        Unavailable,
        Replaced(ProcessIdentity),
        Runtime(ProcessRuntimeError),
    }

    struct ExpectedProcess {
        canonical_executable_path: Option<PathBuf>,
        executable_name: Option<String>,
    }

    impl ProcessRights {
        const SYNCHRONIZE: Self = Self(SYNCHRONIZE_ACCESS);
        const TERMINATE_AND_SYNCHRONIZE: Self = Self(PROCESS_TERMINATE | SYNCHRONIZE_ACCESS);
    }
    const SYNCHRONIZE_ACCESS: u32 = 0x0010_0000;
    const OUTPUT_LIMIT_BYTES: usize = 65_536;

    #[derive(Default)]
    struct OutputBuffer {
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        truncated: bool,
    }

    impl OutputBuffer {
        fn append(&mut self, stdout: bool, bytes: &[u8]) {
            let target = if stdout {
                &mut self.stdout
            } else {
                &mut self.stderr
            };
            if bytes.len() >= OUTPUT_LIMIT_BYTES {
                target.clear();
                target.extend_from_slice(&bytes[bytes.len() - OUTPUT_LIMIT_BYTES..]);
                self.truncated = true;
                return;
            }
            let overflow = target
                .len()
                .saturating_add(bytes.len())
                .saturating_sub(OUTPUT_LIMIT_BYTES);
            if overflow > 0 {
                target.drain(..overflow);
                self.truncated = true;
            }
            target.extend_from_slice(bytes);
        }
    }

    #[derive(Clone)]
    pub(super) struct CapturedOutput {
        buffer: Arc<Mutex<OutputBuffer>>,
    }

    impl CapturedOutput {
        fn from_child(child: &mut std::process::Child) -> Result<Self, ProcessRuntimeError> {
            let stdout = child
                .stdout
                .take()
                .ok_or_else(|| ProcessRuntimeError::new("stdout capture pipe is unavailable"))?;
            let stderr = child
                .stderr
                .take()
                .ok_or_else(|| ProcessRuntimeError::new("stderr capture pipe is unavailable"))?;
            let buffer = Arc::new(Mutex::new(OutputBuffer::default()));
            spawn_output_reader(stdout, Arc::clone(&buffer), true);
            spawn_output_reader(stderr, Arc::clone(&buffer), false);
            Ok(Self { buffer })
        }

        pub(super) fn read(&self) -> ProcessOutput {
            let buffer = self
                .buffer
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            ProcessOutput {
                stdout: String::from_utf8_lossy(&buffer.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&buffer.stderr).into_owned(),
                truncated: buffer.truncated,
            }
        }
    }

    fn spawn_output_reader(
        mut reader: impl Read + Send + 'static,
        buffer: Arc<Mutex<OutputBuffer>>,
        stdout: bool,
    ) {
        thread::spawn(move || {
            let mut bytes = [0_u8; 8_192];
            loop {
                match reader.read(&mut bytes) {
                    Ok(0) | Err(_) => break,
                    Ok(count) => buffer
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .append(stdout, &bytes[..count]),
                }
            }
        });
    }

    pub(super) struct LaunchedProcess {
        pub(super) identity: ProcessIdentity,
        pub(super) captured_output: Option<CapturedOutput>,
    }

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    fn runtime_error(context: &str, error: impl std::fmt::Display) -> ProcessRuntimeError {
        ProcessRuntimeError::new(format!("{context}: {error}"))
    }

    fn direct_executable(recipe: &LaunchRecipe) -> Result<PathBuf, ProcessRuntimeError> {
        match &recipe.source {
            LaunchSource::DirectExecutable { executable_path } => Path::new(executable_path)
                .canonicalize()
                .map_err(|error| runtime_error("executable path is not launchable", error)),
            LaunchSource::Steam { .. } => Err(ProcessRuntimeError::new(
                "Steam launch is not available through the direct executable adapter",
            )),
        }
    }

    fn expected_process(recipe: &LaunchRecipe) -> Result<ExpectedProcess, ProcessRuntimeError> {
        if let Some(monitored_process) = recipe.monitored_process.as_deref() {
            let monitored_name = Path::new(monitored_process);
            if monitored_process.trim().is_empty()
                || monitored_name.file_name().and_then(|name| name.to_str())
                    != Some(monitored_process)
            {
                return Err(ProcessRuntimeError::new(
                    "monitored process must be an executable file name",
                ));
            }
            let canonical_executable_path = recipe
                .monitored_executable_path
                .as_deref()
                .map(Path::new)
                .map(Path::canonicalize)
                .transpose()
                .map_err(|error| {
                    runtime_error("monitored executable path is not accessible", error)
                })?;
            if canonical_executable_path
                .as_deref()
                .and_then(Path::file_name)
                .is_some_and(|name| {
                    !name
                        .to_string_lossy()
                        .eq_ignore_ascii_case(monitored_process)
                })
            {
                return Err(ProcessRuntimeError::new(
                    "monitored executable path does not match the monitored process name",
                ));
            }
            return Ok(ExpectedProcess {
                canonical_executable_path,
                executable_name: Some(monitored_process.to_owned()),
            });
        }

        if recipe.monitored_executable_path.is_some() {
            return Err(ProcessRuntimeError::new(
                "monitored executable path requires a monitored process name",
            ));
        }

        match &recipe.source {
            LaunchSource::DirectExecutable { .. } => {
                direct_executable(recipe).map(|canonical_executable_path| ExpectedProcess {
                    canonical_executable_path: Some(canonical_executable_path),
                    executable_name: None,
                })
            }
            LaunchSource::Steam { .. } => Err(ProcessRuntimeError::new(
                "Steam launch requires a monitored process executable name",
            )),
        }
    }

    fn creation_time_value(creation_time: FILETIME) -> String {
        ((u64::from(creation_time.dwHighDateTime) << 32) | u64::from(creation_time.dwLowDateTime))
            .to_string()
    }

    fn process_creation_time(handle: &OwnedHandle) -> Result<String, ProcessRuntimeError> {
        let mut creation_time = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut exit_time = creation_time;
        let mut kernel_time = creation_time;
        let mut user_time = creation_time;
        if unsafe {
            GetProcessTimes(
                handle.0,
                &mut creation_time,
                &mut exit_time,
                &mut kernel_time,
                &mut user_time,
            )
        } == 0
        {
            return Err(runtime_error(
                "process creation time could not be read",
                io::Error::last_os_error(),
            ));
        }
        Ok(creation_time_value(creation_time))
    }

    fn process_executable_path(handle: &OwnedHandle) -> Result<String, ProcessRuntimeError> {
        let mut path_buffer = vec![0_u16; 32_768];
        let mut path_length = u32::try_from(path_buffer.len())
            .map_err(|_| ProcessRuntimeError::new("process path buffer is too large"))?;
        if unsafe {
            QueryFullProcessImageNameW(handle.0, 0, path_buffer.as_mut_ptr(), &mut path_length)
        } == 0
        {
            return Err(runtime_error(
                "process executable path could not be read",
                io::Error::last_os_error(),
            ));
        }
        path_buffer.truncate(path_length as usize);
        let canonical_executable_path = PathBuf::from(OsString::from_wide(&path_buffer))
            .canonicalize()
            .map_err(|error| {
                runtime_error("process executable path could not canonicalize", error)
            })?
            .to_string_lossy()
            .into_owned();
        Ok(canonical_executable_path)
    }

    fn process_identity_from_handle(
        pid: u32,
        handle: &OwnedHandle,
    ) -> Result<ProcessIdentity, ProcessRuntimeError> {
        Ok(ProcessIdentity {
            pid,
            creation_time: process_creation_time(handle)?,
            canonical_executable_path: process_executable_path(handle)?,
        })
    }

    pub(super) fn process_identity(pid: u32) -> Result<ProcessIdentity, ProcessRuntimeError> {
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            return Err(runtime_error(
                "process identity could not be opened",
                io::Error::last_os_error(),
            ));
        }
        process_identity_from_handle(pid, &OwnedHandle(handle))
    }

    fn with_verified_process<T>(
        identity: &ProcessIdentity,
        required_rights: ProcessRights,
        action: impl FnOnce(&VerifiedProcessHandle) -> Result<T, ProcessRuntimeError>,
    ) -> Result<T, VerifiedProcessError> {
        let handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | required_rights.0,
                0,
                identity.pid,
            )
        };
        if handle.is_null() {
            return Err(VerifiedProcessError::Unavailable);
        }
        let verified = VerifiedProcessHandle {
            handle: OwnedHandle(handle),
        };
        let creation_time =
            process_creation_time(&verified.handle).map_err(VerifiedProcessError::Runtime)?;
        if unsafe { WaitForSingleObject(verified.handle.0, 0) } == WAIT_OBJECT_0 {
            return Err(VerifiedProcessError::Unavailable);
        }
        if creation_time != identity.creation_time {
            return match process_executable_path(&verified.handle) {
                Ok(canonical_executable_path) => {
                    Err(VerifiedProcessError::Replaced(ProcessIdentity {
                        pid: identity.pid,
                        creation_time,
                        canonical_executable_path,
                    }))
                }
                Err(_) => Err(VerifiedProcessError::Unavailable),
            };
        }
        let canonical_executable_path = match process_executable_path(&verified.handle) {
            Ok(path) => path,
            Err(_) => return Err(VerifiedProcessError::Unavailable),
        };
        if !canonical_executable_path.eq_ignore_ascii_case(&identity.canonical_executable_path) {
            return Err(VerifiedProcessError::Replaced(ProcessIdentity {
                pid: identity.pid,
                creation_time,
                canonical_executable_path,
            }));
        }
        action(&verified).map_err(VerifiedProcessError::Runtime)
    }

    fn expected_process_matches(identity: &ProcessIdentity, expected: &ExpectedProcess) -> bool {
        if let Some(expected_path) = expected.canonical_executable_path.as_deref() {
            return Path::new(&identity.canonical_executable_path)
                .to_string_lossy()
                .eq_ignore_ascii_case(&expected_path.to_string_lossy());
        }

        expected.executable_name.as_deref().is_some_and(|name| {
            Path::new(&identity.canonical_executable_path)
                .file_name()
                .is_some_and(|candidate| candidate.to_string_lossy().eq_ignore_ascii_case(name))
        })
    }

    struct WindowObservation {
        pid: u32,
        has_window: bool,
        has_hung_window: bool,
    }

    unsafe extern "system" fn observe_window(window: HWND, context: LPARAM) -> i32 {
        let context = unsafe { &mut *(context as *mut WindowObservation) };
        let mut window_pid = 0;
        unsafe {
            GetWindowThreadProcessId(window, &mut window_pid);
        }
        if window_pid == context.pid && unsafe { IsWindowVisible(window) } != 0 {
            context.has_window = true;
            if unsafe { IsHungAppWindow(window) } != 0 {
                context.has_hung_window = true;
            }
        }
        1
    }

    fn process_responsiveness(pid: u32) -> Result<ProcessResponsiveness, ProcessRuntimeError> {
        let mut observation = WindowObservation {
            pid,
            has_window: false,
            has_hung_window: false,
        };
        unsafe {
            SetLastError(0);
        }
        if unsafe {
            EnumWindows(
                Some(observe_window),
                &mut observation as *mut WindowObservation as LPARAM,
            )
        } == 0
        {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(0) {
                return Err(runtime_error(
                    "process windows could not be observed",
                    error,
                ));
            }
        }

        Ok(if observation.has_hung_window {
            ProcessResponsiveness::NotResponsive
        } else if observation.has_window {
            ProcessResponsiveness::Responsive
        } else {
            ProcessResponsiveness::NotApplicable
        })
    }

    struct CloseWindowRequest {
        pid: u32,
        requested_count: usize,
    }

    unsafe extern "system" fn request_window_close(window: HWND, context: LPARAM) -> i32 {
        let context = unsafe { &mut *(context as *mut CloseWindowRequest) };
        let mut window_pid = 0;
        unsafe {
            GetWindowThreadProcessId(window, &mut window_pid);
        }
        if window_pid == context.pid && unsafe { PostMessageW(window, WM_CLOSE, 0, 0) } != 0 {
            context.requested_count += 1;
        }
        1
    }

    pub(super) fn matching_processes(
        recipe: &LaunchRecipe,
    ) -> Result<Vec<ProcessIdentity>, ProcessRuntimeError> {
        let expected = expected_process(recipe)?;
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(runtime_error(
                "process list could not be opened",
                io::Error::last_os_error(),
            ));
        }
        let snapshot = OwnedHandle(snapshot);
        let mut entry = PROCESSENTRY32W {
            dwSize: u32::try_from(size_of::<PROCESSENTRY32W>())
                .map_err(|_| ProcessRuntimeError::new("process entry size is invalid"))?,
            ..PROCESSENTRY32W::default()
        };
        let mut matches = Vec::new();
        let mut has_entry = unsafe { Process32FirstW(snapshot.0, &mut entry) } != 0;

        while has_entry {
            if let Ok(identity) = process_identity(entry.th32ProcessID)
                && expected_process_matches(&identity, &expected)
            {
                matches.push(identity);
            }
            has_entry = unsafe { Process32NextW(snapshot.0, &mut entry) } != 0;
        }

        matches.sort_by_key(|identity| identity.pid);
        Ok(matches)
    }

    pub(super) fn same_executable_descendants(
        identity: &ProcessIdentity,
    ) -> Result<Vec<ProcessIdentity>, ProcessRuntimeError> {
        // Toolhelp only carries reusable PIDs. Once the launcher has exited we
        // cannot prove an entry still descends from this exact Process, so never
        // turn a child of a PID replacement into Session-owned state.
        if process_identity(identity.pid).ok().as_ref() != Some(identity) {
            return Ok(Vec::new());
        }
        let ancestor_creation_time = identity.creation_time.parse::<u64>().map_err(|_| {
            ProcessRuntimeError::new("process creation time is not a Windows timestamp")
        })?;
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return Err(runtime_error(
                "process list could not be opened",
                io::Error::last_os_error(),
            ));
        }
        let snapshot = OwnedHandle(snapshot);
        let mut entry = PROCESSENTRY32W {
            dwSize: u32::try_from(size_of::<PROCESSENTRY32W>())
                .map_err(|_| ProcessRuntimeError::new("process entry size is invalid"))?,
            ..PROCESSENTRY32W::default()
        };
        let mut entries = Vec::new();
        let mut has_entry = unsafe { Process32FirstW(snapshot.0, &mut entry) } != 0;
        while has_entry {
            entries.push((entry.th32ProcessID, entry.th32ParentProcessID));
            has_entry = unsafe { Process32NextW(snapshot.0, &mut entry) } != 0;
        }

        let mut ancestor_processes = BTreeSet::from([identity.pid]);
        let mut descendants = Vec::new();
        loop {
            let mut found_descendant = false;
            for (pid, parent_pid) in &entries {
                if !ancestor_processes.contains(parent_pid) || ancestor_processes.contains(pid) {
                    continue;
                }
                let Ok(candidate) = process_identity(*pid) else {
                    continue;
                };
                let Ok(candidate_creation_time) = candidate.creation_time.parse::<u64>() else {
                    continue;
                };
                if candidate_creation_time < ancestor_creation_time {
                    continue;
                }
                if !candidate
                    .canonical_executable_path
                    .eq_ignore_ascii_case(&identity.canonical_executable_path)
                {
                    continue;
                }
                ancestor_processes.insert(*pid);
                descendants.push(candidate);
                found_descendant = true;
            }
            if !found_descendant {
                break;
            }
        }
        descendants.sort_by_key(|candidate| candidate.pid);
        Ok(descendants)
    }

    pub(super) fn running_executable_paths() -> Vec<PathBuf> {
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return Vec::new();
        }
        let snapshot = OwnedHandle(snapshot);
        let Ok(entry_size) = u32::try_from(size_of::<PROCESSENTRY32W>()) else {
            return Vec::new();
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: entry_size,
            ..PROCESSENTRY32W::default()
        };
        let mut executable_paths = Vec::new();
        let mut has_entry = unsafe { Process32FirstW(snapshot.0, &mut entry) } != 0;
        while has_entry {
            if let Ok(identity) = process_identity(entry.th32ProcessID) {
                executable_paths.push(PathBuf::from(identity.canonical_executable_path));
            }
            has_entry = unsafe { Process32NextW(snapshot.0, &mut entry) } != 0;
        }
        executable_paths.sort();
        executable_paths.dedup();
        executable_paths
    }

    pub(super) fn launch_direct(
        recipe: &LaunchRecipe,
        capture_output: bool,
    ) -> Result<LaunchedProcess, ProcessRuntimeError> {
        if recipe.elevated {
            return Err(ProcessRuntimeError::new(
                "elevated launch requires the one-shot helper",
            ));
        }
        let executable = direct_executable(recipe)?;
        let working_directory = recipe
            .working_directory
            .as_deref()
            .map(Path::new)
            .map(Path::canonicalize)
            .transpose()
            .map_err(|error| runtime_error("working directory is not accessible", error))?
            .or_else(|| executable.parent().map(Path::to_path_buf))
            .ok_or_else(|| {
                ProcessRuntimeError::new("executable does not have a working directory")
            })?;
        let creation_flags = CREATE_NEW_PROCESS_GROUP
            | if recipe.console_visibility == ConsoleVisibility::Hidden {
                CREATE_NO_WINDOW
            } else {
                0
            };
        let existing_monitored_processes = if recipe.monitored_process.is_some() {
            matching_processes(recipe)?
        } else {
            Vec::new()
        };
        let mut command = Command::new(&executable);
        command
            .args(&recipe.arguments)
            .current_dir(working_directory)
            .creation_flags(creation_flags)
            .stdin(Stdio::null());
        let capture_output =
            capture_output && recipe.console_visibility == ConsoleVisibility::Hidden;
        if capture_output {
            command.stdout(Stdio::piped()).stderr(Stdio::piped());
        } else if recipe.console_visibility == ConsoleVisibility::Hidden {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        } else {
            command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        }
        let mut child = command
            .spawn()
            .map_err(|error| runtime_error("process could not be launched", error))?;
        let captured_output = if capture_output {
            Some(CapturedOutput::from_child(&mut child)?)
        } else {
            None
        };
        if recipe.monitored_process.is_some() {
            let deadline =
                Instant::now() + Duration::from_secs(u64::from(recipe.startup_timeout_seconds));
            loop {
                if let Some(identity) = matching_processes(recipe)?
                    .into_iter()
                    .find(|identity| !existing_monitored_processes.contains(identity))
                {
                    return Ok(LaunchedProcess {
                        identity,
                        captured_output,
                    });
                }
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ProcessRuntimeError::new(
                        "monitored process did not appear before the startup timeout",
                    ));
                }
                thread::sleep(Duration::from_millis(25));
            }
        }

        process_identity(child.id())
            .map(|identity| LaunchedProcess {
                identity,
                captured_output,
            })
            .inspect_err(|_| {
                let _ = child.kill();
                let _ = child.wait();
            })
    }

    fn launch_steam(recipe: &LaunchRecipe) -> Result<LaunchedProcess, ProcessRuntimeError> {
        if recipe.elevated {
            return Err(ProcessRuntimeError::new(
                "elevated launch requires the one-shot helper",
            ));
        }
        let existing_processes = matching_processes(recipe)?;
        let uri =
            crate::launch_recipe::steam_launch_uri(recipe).map_err(ProcessRuntimeError::new)?;
        let wide_uri = OsStr::new(&uri)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let launch_result = unsafe {
            ShellExecuteW(
                ptr::null_mut(),
                ptr::null(),
                wide_uri.as_ptr(),
                ptr::null(),
                ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        if launch_result as isize <= 32 {
            return Err(ProcessRuntimeError::new(format!(
                "Steam launch request was rejected with ShellExecute status {}",
                launch_result as isize
            )));
        }

        let deadline =
            Instant::now() + Duration::from_secs(u64::from(recipe.startup_timeout_seconds));
        loop {
            if let Some(identity) = matching_processes(recipe)?
                .into_iter()
                .find(|identity| !existing_processes.contains(identity))
            {
                return Ok(LaunchedProcess {
                    identity,
                    captured_output: None,
                });
            }
            if Instant::now() >= deadline {
                return Err(ProcessRuntimeError::new(
                    "Steam monitored process did not appear before the startup timeout",
                ));
            }
            thread::sleep(Duration::from_millis(25));
        }
    }

    pub(super) fn launch(recipe: &LaunchRecipe) -> Result<LaunchedProcess, ProcessRuntimeError> {
        match recipe.source {
            LaunchSource::DirectExecutable { .. } => launch_direct(recipe, true),
            LaunchSource::Steam { .. } => launch_steam(recipe),
        }
    }

    pub(super) fn observe(
        identity: &ProcessIdentity,
    ) -> Result<ProcessObservation, ProcessRuntimeError> {
        match with_verified_process(identity, ProcessRights::SYNCHRONIZE, |process| {
            match unsafe { WaitForSingleObject(process.handle.0, 0) } {
                WAIT_OBJECT_0 => Ok(ProcessObservation::Exited),
                WAIT_TIMEOUT => Ok(ProcessObservation::Running {
                    responsiveness: process_responsiveness(identity.pid)?,
                }),
                _ => Err(runtime_error(
                    "process status could not be observed",
                    io::Error::last_os_error(),
                )),
            }
        }) {
            Ok(observation) => Ok(observation),
            Err(VerifiedProcessError::Unavailable) => Ok(ProcessObservation::Exited),
            Err(VerifiedProcessError::Replaced(current_identity)) => {
                Ok(ProcessObservation::Replaced { current_identity })
            }
            Err(VerifiedProcessError::Runtime(error)) => Err(error),
        }
    }

    pub(super) fn request_graceful_stop(
        identity: &ProcessIdentity,
        strategy: &crate::ShutdownStrategy,
    ) -> Result<super::GracefulStopResult, ProcessRuntimeError> {
        match with_verified_process(
            identity,
            ProcessRights::SYNCHRONIZE,
            |_process| match strategy {
                crate::ShutdownStrategy::CloseWindows => {
                    let mut request = CloseWindowRequest {
                        pid: identity.pid,
                        requested_count: 0,
                    };
                    if unsafe {
                        EnumWindows(
                            Some(request_window_close),
                            &mut request as *mut CloseWindowRequest as LPARAM,
                        )
                    } == 0
                    {
                        return Err(runtime_error(
                            "process windows could not be closed",
                            io::Error::last_os_error(),
                        ));
                    }
                    Ok(if request.requested_count == 0 {
                        super::GracefulStopResult::Unavailable
                    } else {
                        super::GracefulStopResult::Requested
                    })
                }
                crate::ShutdownStrategy::CustomStop {
                    executable_path,
                    arguments,
                } => {
                    let executable =
                        Path::new(executable_path).canonicalize().map_err(|error| {
                            runtime_error("custom stop executable is not launchable", error)
                        })?;
                    let working_directory = executable.parent().ok_or_else(|| {
                        ProcessRuntimeError::new(
                            "custom stop executable does not have a working directory",
                        )
                    })?;
                    let status = Command::new(&executable)
                        .args(arguments)
                        .current_dir(working_directory)
                        .creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                        .map_err(|error| runtime_error("custom stop could not run", error))?;
                    if !status.success() {
                        return Err(ProcessRuntimeError::new(format!(
                            "custom stop exited unsuccessfully with {status}"
                        )));
                    }
                    Ok(super::GracefulStopResult::Requested)
                }
                crate::ShutdownStrategy::ConsoleInterrupt => {
                    if unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, identity.pid) } == 0 {
                        return Ok(super::GracefulStopResult::Unavailable);
                    }
                    Ok(super::GracefulStopResult::Requested)
                }
                crate::ShutdownStrategy::ForceOnly => Ok(super::GracefulStopResult::Unavailable),
            },
        ) {
            Ok(result) => Ok(result),
            Err(VerifiedProcessError::Unavailable | VerifiedProcessError::Replaced(_)) => {
                Ok(super::GracefulStopResult::Unavailable)
            }
            Err(VerifiedProcessError::Runtime(error)) => Err(error),
        }
    }

    pub(super) fn wait_for_exit(
        identity: &ProcessIdentity,
        timeout: Duration,
    ) -> Result<bool, ProcessRuntimeError> {
        match with_verified_process(identity, ProcessRights::SYNCHRONIZE, |process| {
            let timeout_milliseconds = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX);
            match unsafe { WaitForSingleObject(process.handle.0, timeout_milliseconds) } {
                WAIT_OBJECT_0 => Ok(true),
                WAIT_TIMEOUT => Ok(false),
                _ => Err(runtime_error(
                    "process exit could not be awaited",
                    io::Error::last_os_error(),
                )),
            }
        }) {
            Ok(exited) => Ok(exited),
            Err(VerifiedProcessError::Unavailable | VerifiedProcessError::Replaced(_)) => Ok(true),
            Err(VerifiedProcessError::Runtime(error)) => Err(error),
        }
    }

    pub(super) fn force_stop(identity: &ProcessIdentity) -> Result<(), ProcessRuntimeError> {
        match with_verified_process(
            identity,
            ProcessRights::TERMINATE_AND_SYNCHRONIZE,
            |process| {
                if unsafe { TerminateProcess(process.handle.0, 1) } == 0 {
                    return Err(runtime_error(
                        "process could not be force stopped",
                        io::Error::last_os_error(),
                    ));
                }
                match unsafe { WaitForSingleObject(process.handle.0, 5_000) } {
                    WAIT_OBJECT_0 => Ok(()),
                    WAIT_TIMEOUT => Err(ProcessRuntimeError::new(
                        "force-stopped process did not exit within five seconds",
                    )),
                    _ => Err(runtime_error(
                        "force-stopped process exit could not be awaited",
                        io::Error::last_os_error(),
                    )),
                }
            },
        ) {
            Ok(()) => Ok(()),
            Err(VerifiedProcessError::Unavailable | VerifiedProcessError::Replaced(_)) => Ok(()),
            Err(VerifiedProcessError::Runtime(error)) => Err(error),
        }
    }
}
