use formation_lap_lib::{
    AppCommand, ApplicationProcessSnapshot, CommandOutcome, FormationLapCore, GracefulStopResult,
    LaunchRecipe, LaunchSource, ProcessIdentity, ProcessObservation, ProcessOutput,
    ProcessOwnership, ProcessResponsiveness, ProcessRuntime, ProcessRuntimeError, ProcessStatus,
    ShutdownStrategy,
};
use std::{
    collections::VecDeque,
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
    matching_processes: VecDeque<Vec<ProcessIdentity>>,
    launch_results: VecDeque<Result<ProcessIdentity, ProcessRuntimeError>>,
    observations: VecDeque<ProcessObservation>,
    graceful_stop_results: VecDeque<Result<GracefulStopResult, ProcessRuntimeError>>,
    wait_for_exit_results: VecDeque<Result<bool, ProcessRuntimeError>>,
    force_stop_results: VecDeque<Result<(), ProcessRuntimeError>>,
}

impl ProcessRuntime for ScriptedProcessRuntime {
    fn matching_processes(
        &mut self,
        _recipe: &LaunchRecipe,
    ) -> Result<Vec<ProcessIdentity>, ProcessRuntimeError> {
        Ok(self.matching_processes.pop_front().unwrap_or_default())
    }

    fn launch(&mut self, _recipe: &LaunchRecipe) -> Result<ProcessIdentity, ProcessRuntimeError> {
        self.launch_results
            .pop_front()
            .ok_or_else(|| ProcessRuntimeError::new("the launch script is exhausted"))?
    }

    fn observe(
        &mut self,
        _identity: &ProcessIdentity,
    ) -> Result<ProcessObservation, ProcessRuntimeError> {
        self.observations
            .pop_front()
            .ok_or_else(|| ProcessRuntimeError::new("the observation script is exhausted"))
    }

    fn request_graceful_stop(
        &mut self,
        _identity: &ProcessIdentity,
        _strategy: &ShutdownStrategy,
    ) -> Result<GracefulStopResult, ProcessRuntimeError> {
        self.graceful_stop_results
            .pop_front()
            .ok_or_else(|| ProcessRuntimeError::new("the graceful-stop script is exhausted"))?
    }

    fn wait_for_exit(
        &mut self,
        _identity: &ProcessIdentity,
        _timeout: Duration,
    ) -> Result<bool, ProcessRuntimeError> {
        self.wait_for_exit_results
            .pop_front()
            .ok_or_else(|| ProcessRuntimeError::new("the exit-wait script is exhausted"))?
    }

    fn force_stop(&mut self, _identity: &ProcessIdentity) -> Result<(), ProcessRuntimeError> {
        self.force_stop_results
            .pop_front()
            .ok_or_else(|| ProcessRuntimeError::new("force stop must not be requested"))?
    }

    fn read_output(
        &mut self,
        _identity: &ProcessIdentity,
    ) -> Result<ProcessOutput, ProcessRuntimeError> {
        Ok(ProcessOutput::default())
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
        matching_processes: VecDeque::new(),
        launch_results: VecDeque::from([Ok(launched_identity.clone())]),
        observations: VecDeque::new(),
        graceful_stop_results: VecDeque::new(),
        wait_for_exit_results: VecDeque::new(),
        force_stop_results: VecDeque::new(),
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
            output: None,
        }]
    );
}

#[test]
fn filename_only_launcher_identity_is_observed_without_session_ownership() {
    let storage = TempStorage::new();
    let launched_identity = ProcessIdentity {
        pid: 4_913,
        creation_time: "133822944050000000".to_owned(),
        canonical_executable_path: r"C:\Games\Observed\sim.exe".to_owned(),
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::new(),
        launch_results: VecDeque::from([Ok(launched_identity.clone())]),
        observations: VecDeque::new(),
        graceful_stop_results: VecDeque::new(),
        wait_for_exit_results: VecDeque::new(),
        force_stop_results: VecDeque::new(),
    };
    let (mut core, profile_id, application_id) = configured_core(&storage, runtime);
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("configured profile should be selected");
    profile.primary_sim.launch_recipe.monitored_process = Some("sim.exe".to_owned());
    profile.primary_sim.launch_recipe.monitored_executable_path = None;
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("filename-only launcher recipe should remain readable");

    let outcome = core
        .execute(AppCommand::StartApplication {
            profile_id,
            application_id: application_id.clone(),
        })
        .expect("filename-only launcher Process should remain observable");

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
            identity: Some(launched_identity),
            output: None,
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
        matching_processes: VecDeque::from([vec![existing_identity.clone()]]),
        launch_results: VecDeque::from([Err(ProcessRuntimeError::new(
            "a duplicate process must not be launched",
        ))]),
        observations: VecDeque::new(),
        graceful_stop_results: VecDeque::new(),
        wait_for_exit_results: VecDeque::new(),
        force_stop_results: VecDeque::new(),
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
            output: None,
        }]
    );
}

#[test]
fn pre_existing_exit_and_restart_do_not_request_shutdown_without_confirmation() {
    let storage = TempStorage::new();
    let existing_identity = ProcessIdentity {
        pid: 5_348,
        creation_time: "133822944150000000".to_owned(),
        canonical_executable_path: std::env::current_exe()
            .expect("test executable path should be available")
            .canonicalize()
            .expect("test executable path should canonicalize")
            .to_string_lossy()
            .into_owned(),
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::from([vec![existing_identity.clone()]]),
        launch_results: VecDeque::new(),
        observations: VecDeque::new(),
        graceful_stop_results: VecDeque::new(),
        wait_for_exit_results: VecDeque::new(),
        force_stop_results: VecDeque::new(),
    };
    let (mut core, profile_id, application_id) = configured_core(&storage, runtime);
    core.execute(AppCommand::StartApplication {
        profile_id: profile_id.clone(),
        application_id: application_id.clone(),
    })
    .expect("the Pre-existing Process should be observed");

    for outcome in [
        core.execute(AppCommand::ExitApplication {
            application_id: application_id.clone(),
            pre_existing_confirmed: false,
        })
        .expect("unconfirmed Exit should be rejected safely"),
        core.execute(AppCommand::RestartApplication {
            profile_id,
            application_id: application_id.clone(),
            pre_existing_confirmed: false,
        })
        .expect("unconfirmed Restart should be rejected safely"),
    ] {
        assert_eq!(
            outcome,
            CommandOutcome::PreExistingControlConfirmationRequired {
                application_id: application_id.clone(),
            }
        );
    }
    assert_eq!(
        core.snapshot().application_processes,
        vec![ApplicationProcessSnapshot {
            application_id,
            status: ProcessStatus::RunningPreExisting,
            ownership: Some(ProcessOwnership::PreExisting),
            identity: Some(existing_identity),
            output: None,
        }]
    );
}

#[test]
fn two_failed_window_checks_mark_not_responding_and_a_successful_check_recovers() {
    let storage = TempStorage::new();
    let launched_identity = ProcessIdentity {
        pid: 5_612,
        creation_time: "133822944200000000".to_owned(),
        canonical_executable_path: std::env::current_exe()
            .expect("test executable path should be available")
            .canonicalize()
            .expect("test executable path should canonicalize")
            .to_string_lossy()
            .into_owned(),
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::new(),
        launch_results: VecDeque::from([Ok(launched_identity)]),
        observations: VecDeque::from([
            ProcessObservation::Running {
                responsiveness: ProcessResponsiveness::NotResponsive,
            },
            ProcessObservation::Running {
                responsiveness: ProcessResponsiveness::NotResponsive,
            },
            ProcessObservation::Running {
                responsiveness: ProcessResponsiveness::Responsive,
            },
        ]),
        graceful_stop_results: VecDeque::new(),
        wait_for_exit_results: VecDeque::new(),
        force_stop_results: VecDeque::new(),
    };
    let (mut core, profile_id, application_id) = configured_core(&storage, runtime);
    core.execute(AppCommand::StartApplication {
        profile_id,
        application_id: application_id.clone(),
    })
    .expect("configured application should start");

    core.execute(AppCommand::RefreshProcesses)
        .expect("the first window check should complete");
    assert_eq!(
        core.snapshot().application_processes[0].status,
        ProcessStatus::Running
    );

    core.execute(AppCommand::RefreshProcesses)
        .expect("the second window check should complete");
    assert_eq!(
        core.snapshot().application_processes[0].status,
        ProcessStatus::NotResponding
    );

    core.execute(AppCommand::RefreshProcesses)
        .expect("the recovery check should complete");
    assert_eq!(
        core.snapshot().application_processes[0].status,
        ProcessStatus::Running
    );
}

#[test]
fn a_process_that_exits_while_starting_is_reported_as_failed() {
    let storage = TempStorage::new();
    let launched_identity = ProcessIdentity {
        pid: 5_868,
        creation_time: "133822944250000000".to_owned(),
        canonical_executable_path: std::env::current_exe()
            .expect("test executable path should be available")
            .canonicalize()
            .expect("test executable path should canonicalize")
            .to_string_lossy()
            .into_owned(),
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::new(),
        launch_results: VecDeque::from([Ok(launched_identity)]),
        observations: VecDeque::from([ProcessObservation::Exited]),
        graceful_stop_results: VecDeque::new(),
        wait_for_exit_results: VecDeque::new(),
        force_stop_results: VecDeque::new(),
    };
    let (mut core, profile_id, application_id) = configured_core(&storage, runtime);
    core.execute(AppCommand::StartApplication {
        profile_id,
        application_id: application_id.clone(),
    })
    .expect("configured application should start");

    core.execute(AppCommand::RefreshProcesses)
        .expect("the failed startup should be observed");

    assert_eq!(
        core.snapshot().application_processes,
        vec![ApplicationProcessSnapshot {
            application_id,
            status: ProcessStatus::Failed,
            ownership: None,
            identity: None,
            output: None,
        }]
    );
}

#[test]
fn exit_requests_graceful_shutdown_before_recording_a_clean_stop() {
    let storage = TempStorage::new();
    let launched_identity = ProcessIdentity {
        pid: 6_124,
        creation_time: "133822944300000000".to_owned(),
        canonical_executable_path: std::env::current_exe()
            .expect("test executable path should be available")
            .canonicalize()
            .expect("test executable path should canonicalize")
            .to_string_lossy()
            .into_owned(),
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::new(),
        launch_results: VecDeque::from([Ok(launched_identity)]),
        observations: VecDeque::new(),
        graceful_stop_results: VecDeque::from([Ok(GracefulStopResult::Requested)]),
        wait_for_exit_results: VecDeque::from([Ok(true)]),
        force_stop_results: VecDeque::new(),
    };
    let (mut core, profile_id, application_id) = configured_core(&storage, runtime);
    core.execute(AppCommand::StartApplication {
        profile_id,
        application_id: application_id.clone(),
    })
    .expect("configured application should start");

    let outcome = core
        .execute(AppCommand::ExitApplication {
            application_id: application_id.clone(),
            pre_existing_confirmed: false,
        })
        .expect("graceful shutdown should complete");

    assert_eq!(
        outcome,
        CommandOutcome::ApplicationStopped {
            application_id: application_id.clone(),
        }
    );
    assert_eq!(
        core.snapshot().application_processes,
        vec![ApplicationProcessSnapshot {
            application_id,
            status: ProcessStatus::Stopped,
            ownership: None,
            identity: None,
            output: None,
        }]
    );
}

#[test]
fn force_stop_requires_explicit_confirmation_after_graceful_timeout() {
    let storage = TempStorage::new();
    let launched_identity = ProcessIdentity {
        pid: 6_636,
        creation_time: "133822944400000000".to_owned(),
        canonical_executable_path: std::env::current_exe()
            .expect("test executable path should be available")
            .canonicalize()
            .expect("test executable path should canonicalize")
            .to_string_lossy()
            .into_owned(),
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::new(),
        launch_results: VecDeque::from([Ok(launched_identity)]),
        observations: VecDeque::new(),
        graceful_stop_results: VecDeque::from([Ok(GracefulStopResult::Requested)]),
        wait_for_exit_results: VecDeque::from([Ok(false)]),
        force_stop_results: VecDeque::from([Ok(())]),
    };
    let (mut core, profile_id, application_id) = configured_core(&storage, runtime);
    core.execute(AppCommand::StartApplication {
        profile_id,
        application_id: application_id.clone(),
    })
    .expect("configured application should start");

    let outcome = core
        .execute(AppCommand::ExitApplication {
            application_id: application_id.clone(),
            pre_existing_confirmed: false,
        })
        .expect("graceful timeout should be reported");
    assert_eq!(
        outcome,
        CommandOutcome::ForceStopConfirmationRequired {
            application_id: application_id.clone(),
        }
    );
    assert_eq!(
        core.snapshot().application_processes[0].status,
        ProcessStatus::Stopping
    );

    let unconfirmed = core
        .execute(AppCommand::ForceStopApplication {
            application_id: application_id.clone(),
            pre_existing_confirmed: false,
            force_confirmed: false,
        })
        .expect("an unconfirmed force request should be rejected safely");
    assert_eq!(
        unconfirmed,
        CommandOutcome::ForceStopConfirmationRequired {
            application_id: application_id.clone(),
        }
    );

    let confirmed = core
        .execute(AppCommand::ForceStopApplication {
            application_id: application_id.clone(),
            pre_existing_confirmed: false,
            force_confirmed: true,
        })
        .expect("a confirmed force request should stop the Process");
    assert_eq!(
        confirmed,
        CommandOutcome::ApplicationStopped {
            application_id: application_id.clone(),
        }
    );
    assert_eq!(
        core.snapshot().application_processes[0],
        ApplicationProcessSnapshot {
            application_id,
            status: ProcessStatus::Stopped,
            ownership: None,
            identity: None,
            output: None,
        }
    );
}

#[test]
fn restart_waits_for_the_old_process_to_exit_before_starting_one_replacement() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize")
        .to_string_lossy()
        .into_owned();
    let first_identity = ProcessIdentity {
        pid: 7_148,
        creation_time: "133822944500000000".to_owned(),
        canonical_executable_path: executable_path.clone(),
    };
    let replacement_identity = ProcessIdentity {
        pid: 7_660,
        creation_time: "133822944600000000".to_owned(),
        canonical_executable_path: executable_path,
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::new(),
        launch_results: VecDeque::from([Ok(first_identity), Ok(replacement_identity.clone())]),
        observations: VecDeque::new(),
        graceful_stop_results: VecDeque::from([Ok(GracefulStopResult::Requested)]),
        wait_for_exit_results: VecDeque::from([Ok(true)]),
        force_stop_results: VecDeque::new(),
    };
    let (mut core, profile_id, application_id) = configured_core(&storage, runtime);
    core.execute(AppCommand::StartApplication {
        profile_id: profile_id.clone(),
        application_id: application_id.clone(),
    })
    .expect("configured application should start");

    let outcome = core
        .execute(AppCommand::RestartApplication {
            profile_id,
            application_id: application_id.clone(),
            pre_existing_confirmed: false,
        })
        .expect("the application should restart after its old Process exits");

    assert_eq!(
        outcome,
        CommandOutcome::ApplicationRestarted {
            application_id: application_id.clone(),
        }
    );
    assert_eq!(
        core.snapshot().application_processes,
        vec![ApplicationProcessSnapshot {
            application_id,
            status: ProcessStatus::Starting,
            ownership: Some(ProcessOwnership::SessionOwned),
            identity: Some(replacement_identity),
            output: None,
        }]
    );
}

#[test]
fn restart_adopts_a_racing_matching_process_instead_of_launching_a_duplicate() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize")
        .to_string_lossy()
        .into_owned();
    let first_identity = ProcessIdentity {
        pid: 8_172,
        creation_time: "133822944700000000".to_owned(),
        canonical_executable_path: executable_path.clone(),
    };
    let racing_identity = ProcessIdentity {
        pid: 8_684,
        creation_time: "133822944800000000".to_owned(),
        canonical_executable_path: executable_path,
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::from([Vec::new(), vec![racing_identity.clone()]]),
        launch_results: VecDeque::from([
            Ok(first_identity),
            Err(ProcessRuntimeError::new(
                "restart must not launch beside a racing match",
            )),
        ]),
        observations: VecDeque::new(),
        graceful_stop_results: VecDeque::from([Ok(GracefulStopResult::Requested)]),
        wait_for_exit_results: VecDeque::from([Ok(true)]),
        force_stop_results: VecDeque::new(),
    };
    let (mut core, profile_id, application_id) = configured_core(&storage, runtime);
    core.execute(AppCommand::StartApplication {
        profile_id: profile_id.clone(),
        application_id: application_id.clone(),
    })
    .expect("configured application should start");

    let outcome = core
        .execute(AppCommand::RestartApplication {
            profile_id,
            application_id: application_id.clone(),
            pre_existing_confirmed: false,
        })
        .expect("the racing Process should be adopted safely");

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
            identity: Some(racing_identity),
            output: None,
        }]
    );
}
