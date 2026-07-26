use formation_lap_lib::{
    ApplicationTargetPayload, ConsoleVisibility, CreateProfilePayload, GracefulStopResult,
    LaunchRecipe, LaunchSource, NativeCommandHost, NewRacingProfile, ProcessIdentity,
    ProcessObservation, ProcessOutput, ProcessOwnership, ProcessResponsiveness, ProcessRuntime,
    ProcessRuntimeError, ProcessStatus, SaveProfilePayload, ShutdownStrategy,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TempStorage {
    path: PathBuf,
}

impl TempStorage {
    fn new() -> Self {
        let unique = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "formation-lap-application-command-test-{}-{timestamp}-{unique}",
            process::id()
        ));
        fs::create_dir_all(&path).expect("temporary command storage should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempStorage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct LaunchingRuntime {
    identity: ProcessIdentity,
}

impl ProcessRuntime for LaunchingRuntime {
    fn matching_processes(
        &mut self,
        _recipe: &LaunchRecipe,
    ) -> Result<Vec<ProcessIdentity>, ProcessRuntimeError> {
        Ok(Vec::new())
    }

    fn launch(&mut self, _recipe: &LaunchRecipe) -> Result<ProcessIdentity, ProcessRuntimeError> {
        Ok(self.identity.clone())
    }

    fn observe(
        &mut self,
        _identity: &ProcessIdentity,
    ) -> Result<ProcessObservation, ProcessRuntimeError> {
        Ok(ProcessObservation::Running {
            responsiveness: ProcessResponsiveness::NotApplicable,
        })
    }

    fn request_graceful_stop(
        &mut self,
        _identity: &ProcessIdentity,
        _strategy: &ShutdownStrategy,
    ) -> Result<GracefulStopResult, ProcessRuntimeError> {
        Ok(GracefulStopResult::Unavailable)
    }

    fn wait_for_exit(
        &mut self,
        _identity: &ProcessIdentity,
        _timeout: Duration,
    ) -> Result<bool, ProcessRuntimeError> {
        Ok(false)
    }

    fn force_stop(&mut self, _identity: &ProcessIdentity) -> Result<(), ProcessRuntimeError> {
        Ok(())
    }

    fn read_output(
        &mut self,
        _identity: &ProcessIdentity,
    ) -> Result<ProcessOutput, ProcessRuntimeError> {
        Ok(ProcessOutput::default())
    }
}

#[test]
fn start_application_command_returns_authoritative_native_process_state() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let identity = ProcessIdentity {
        pid: 8_172,
        creation_time: "133822944700000000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let commands = NativeCommandHost::open_with_runtime(
        storage.path(),
        LaunchingRuntime {
            identity: identity.clone(),
        },
    )
    .expect("native command host should open");
    let mut profile = commands
        .create_profile(CreateProfilePayload {
            profile: NewRacingProfile::from_names(
                "Command fixture".to_owned(),
                "Healthy fixture".to_owned(),
            ),
        })
        .expect("fixture profile should be created")
        .selected_profile
        .expect("fixture profile should be selected");
    profile.primary_sim.launch_recipe = LaunchRecipe {
        source: LaunchSource::DirectExecutable {
            executable_path: executable_path.to_string_lossy().into_owned(),
        },
        arguments: Vec::new(),
        working_directory: None,
        monitored_process: None,
        monitored_executable_path: None,
        console_visibility: ConsoleVisibility::Hidden,
        elevated: false,
        startup_timeout_seconds: 3,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::CloseWindows,
    };
    let profile_id = profile.id.clone();
    let application_id = profile.primary_sim.id.clone();
    commands
        .save_profile(SaveProfilePayload { profile })
        .expect("fixture profile should be configured");

    let snapshot = commands
        .start_application(ApplicationTargetPayload {
            profile_id,
            application_id: application_id.clone(),
        })
        .expect("application command should adapt into FormationLapCore");

    assert_eq!(snapshot.application_processes.len(), 1);
    assert_eq!(
        snapshot.application_processes[0].application_id,
        application_id
    );
    assert_eq!(
        snapshot.application_processes[0].status,
        ProcessStatus::Starting
    );
    assert_eq!(
        snapshot.application_processes[0].ownership,
        Some(ProcessOwnership::SessionOwned)
    );
    assert_eq!(snapshot.application_processes[0].identity, Some(identity));
}
