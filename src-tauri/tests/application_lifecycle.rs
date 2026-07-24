use formation_lap_lib::{
    AppCommand, ApplicationProcessSnapshot, CommandOutcome, FormationLapCore, LaunchRecipe,
    LaunchSource, ProcessIdentity, ProcessOwnership, ProcessRuntime, ProcessRuntimeError,
    ProcessStatus,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
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
            "formation-lap-lifecycle-test-{}-{timestamp}-{unique}",
            process::id()
        ));

        fs::create_dir_all(&path).expect("temporary lifecycle storage should be created");
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

struct ScriptedProcessRuntime {
    matching_processes: Vec<ProcessIdentity>,
    launch_result: Result<ProcessIdentity, ProcessRuntimeError>,
}

impl ProcessRuntime for ScriptedProcessRuntime {
    fn matching_processes(
        &mut self,
        _recipe: &LaunchRecipe,
    ) -> Result<Vec<ProcessIdentity>, ProcessRuntimeError> {
        Ok(self.matching_processes.clone())
    }

    fn launch(&mut self, _recipe: &LaunchRecipe) -> Result<ProcessIdentity, ProcessRuntimeError> {
        self.launch_result.clone()
    }
}

fn configured_core(
    storage: &TempStorage,
    runtime: ScriptedProcessRuntime,
) -> (FormationLapCore, String, String) {
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("empty lifecycle storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Local fixture".to_owned(),
            primary_sim_name: "Healthy fixture".to_owned(),
        })
        .expect("fixture profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("created profile should be selected");
    let application_id = profile.primary_sim.id.clone();
    profile.primary_sim.launch_recipe.source = LaunchSource::DirectExecutable {
        executable_path: executable_path.to_string_lossy().into_owned(),
    };
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("fixture profile should be configured");

    (core, profile_id, application_id)
}

#[test]
fn starting_a_configured_application_records_session_owned_stable_identity() {
    let storage = TempStorage::new();
    let launched_identity = ProcessIdentity {
        pid: 4_812,
        creation_time: "133822944000000000".to_owned(),
        canonical_executable_path: std::env::current_exe()
            .expect("test executable path should be available")
            .canonicalize()
            .expect("test executable path should canonicalize")
            .to_string_lossy()
            .into_owned(),
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: Vec::new(),
        launch_result: Ok(launched_identity.clone()),
    };
    let (mut core, profile_id, application_id) = configured_core(&storage, runtime);

    let outcome = core
        .execute(AppCommand::StartApplication {
            profile_id,
            application_id: application_id.clone(),
        })
        .expect("configured application should start");

    assert_eq!(
        outcome,
        CommandOutcome::ApplicationStartRequested {
            application_id: application_id.clone(),
        }
    );
    assert_eq!(
        core.snapshot().application_processes,
        vec![ApplicationProcessSnapshot {
            application_id,
            status: ProcessStatus::Starting,
            ownership: Some(ProcessOwnership::SessionOwned),
            identity: Some(launched_identity),
        }]
    );
}

#[test]
fn starting_an_already_running_application_observes_it_without_taking_ownership() {
    let storage = TempStorage::new();
    let existing_identity = ProcessIdentity {
        pid: 5_104,
        creation_time: "133822944100000000".to_owned(),
        canonical_executable_path: std::env::current_exe()
            .expect("test executable path should be available")
            .canonicalize()
            .expect("test executable path should canonicalize")
            .to_string_lossy()
            .into_owned(),
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: vec![existing_identity.clone()],
        launch_result: Err(ProcessRuntimeError::new(
            "a duplicate process must not be launched",
        )),
    };
    let (mut core, profile_id, application_id) = configured_core(&storage, runtime);

    let outcome = core
        .execute(AppCommand::StartApplication {
            profile_id,
            application_id: application_id.clone(),
        })
        .expect("a Pre-existing Process should be observed");

    assert_eq!(
        outcome,
        CommandOutcome::ApplicationAlreadyRunning {
            application_id: application_id.clone(),
        }
    );
    assert_eq!(
        core.snapshot().application_processes,
        vec![ApplicationProcessSnapshot {
            application_id,
            status: ProcessStatus::RunningPreExisting,
            ownership: Some(ProcessOwnership::PreExisting),
            identity: Some(existing_identity),
        }]
    );
}
