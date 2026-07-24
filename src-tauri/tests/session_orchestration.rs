use formation_lap_lib::{
    AppCommand, ApplicationRequirement, CommandOutcome, ConsoleVisibility, CoreError,
    CreateProfilePayload, FormationLapCore, GracefulStopResult, LaunchRecipe, LaunchSource,
    NativeCommandHost, ProcessIdentity, ProcessObservation, ProcessOutput, ProcessOwnership,
    ProcessRuntime, ProcessRuntimeError, ProcessStatus, ProfileApplication, ProfileIdPayload,
    SaveProfilePayload, SessionApplicationRole, SessionApplicationSnapshot,
    SessionApplicationState, SessionEvent, SessionEventKind, SessionState, SessionSummary,
    ShutdownStrategy, SupportingApplication,
};
use std::{
    collections::{BTreeMap, VecDeque},
    fs,
    path::{Path, PathBuf},
    process,
    sync::{
        Arc, Barrier, Mutex,
        atomic::{AtomicU64, Ordering},
    },
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
            "formation-lap-session-test-{}-{timestamp}-{unique}",
            process::id()
        ));
        fs::create_dir_all(&path).expect("temporary Session storage should be created");
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

#[derive(Default)]
struct ScriptedProcessRuntime {
    matching_processes: VecDeque<Vec<ProcessIdentity>>,
    launch_results: VecDeque<Result<ProcessIdentity, ProcessRuntimeError>>,
    observations: VecDeque<ProcessObservation>,
    observations_by_pid: BTreeMap<u32, VecDeque<ProcessObservation>>,
    graceful_stop_results: VecDeque<Result<GracefulStopResult, ProcessRuntimeError>>,
    wait_for_exit_results: VecDeque<Result<bool, ProcessRuntimeError>>,
    stop_trace: Option<Arc<Mutex<Vec<u32>>>>,
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
        identity: &ProcessIdentity,
    ) -> Result<ProcessObservation, ProcessRuntimeError> {
        if let Some(observation) = self
            .observations_by_pid
            .get_mut(&identity.pid)
            .and_then(VecDeque::pop_front)
        {
            return Ok(observation);
        }
        Ok(self
            .observations
            .pop_front()
            .unwrap_or(ProcessObservation::Running {
                responsiveness: formation_lap_lib::ProcessResponsiveness::NotApplicable,
            }))
    }

    fn request_graceful_stop(
        &mut self,
        identity: &ProcessIdentity,
        _strategy: &ShutdownStrategy,
    ) -> Result<GracefulStopResult, ProcessRuntimeError> {
        if let Some(trace) = &self.stop_trace {
            trace
                .lock()
                .expect("stop trace should not be poisoned")
                .push(identity.pid);
        }
        self.graceful_stop_results
            .pop_front()
            .unwrap_or_else(|| Err(ProcessRuntimeError::new("the stop script is exhausted")))
    }

    fn wait_for_exit(
        &mut self,
        _identity: &ProcessIdentity,
        _timeout: Duration,
    ) -> Result<bool, ProcessRuntimeError> {
        self.wait_for_exit_results.pop_front().unwrap_or_else(|| {
            Err(ProcessRuntimeError::new(
                "the exit-wait script is exhausted",
            ))
        })
    }

    fn force_stop(&mut self, _identity: &ProcessIdentity) -> Result<(), ProcessRuntimeError> {
        Err(ProcessRuntimeError::new(
            "force stop is not part of this tracer",
        ))
    }

    fn read_output(
        &mut self,
        _identity: &ProcessIdentity,
    ) -> Result<ProcessOutput, ProcessRuntimeError> {
        Ok(ProcessOutput::default())
    }
}

fn application(id: &str, name: &str, executable_path: &Path) -> ProfileApplication {
    ProfileApplication {
        id: id.to_owned(),
        name: name.to_owned(),
        launch_recipe: LaunchRecipe {
            source: LaunchSource::DirectExecutable {
                executable_path: executable_path.to_string_lossy().into_owned(),
            },
            arguments: Vec::new(),
            working_directory: executable_path
                .parent()
                .map(|parent| parent.to_string_lossy().into_owned()),
            monitored_process: None,
            console_visibility: ConsoleVisibility::Hidden,
            elevated: false,
            startup_timeout_seconds: 30,
            post_start_delay_milliseconds: 0,
            shutdown_strategy: ShutdownStrategy::CloseWindows,
        },
        path_needs_repair: false,
    }
}

#[test]
fn start_session_enters_starting_and_launches_only_the_first_supporting_application() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let first_identity = ProcessIdentity {
        pid: 9_196,
        creation_time: "133822944900000000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::from([Vec::new()]),
        launch_results: VecDeque::from([Ok(first_identity.clone())]),
        ..ScriptedProcessRuntime::default()
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("empty Session storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Endurance".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
        })
        .expect("fixture profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("fixture profile should be selected");
    profile.supporting_applications = vec![
        SupportingApplication {
            application: application("new-crew-chief", "Crew Chief", &executable_path),
            requirement: ApplicationRequirement::Required,
            keep_running: false,
        },
        SupportingApplication {
            application: application("new-simhub", "SimHub", &executable_path),
            requirement: ApplicationRequirement::Optional,
            keep_running: false,
        },
    ];
    profile.primary_sim = application(
        &profile.primary_sim.id,
        "Le Mans Ultimate",
        &executable_path,
    );
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("fixture profile should be configured");
    let configured_profile = core
        .snapshot()
        .selected_profile
        .expect("configured profile should remain selected");
    let crew_chief_id = configured_profile.supporting_applications[0]
        .application
        .id
        .clone();
    let simhub_id = configured_profile.supporting_applications[1]
        .application
        .id
        .clone();
    let primary_sim_id = configured_profile.primary_sim.id.clone();

    let outcome = core
        .execute(AppCommand::StartSession {
            profile_id: profile_id.clone(),
        })
        .expect("Idle should accept Start Session");

    assert_eq!(
        outcome,
        CommandOutcome::SessionStartRequested {
            profile_id: profile_id.clone(),
        }
    );
    let snapshot = core.snapshot();
    assert_eq!(snapshot.session.state, SessionState::Starting);
    assert_eq!(snapshot.session.active_profile_id, Some(profile_id));
    assert_eq!(
        snapshot.session.applications,
        vec![
            SessionApplicationSnapshot {
                application_id: crew_chief_id.clone(),
                name: "Crew Chief".to_owned(),
                role: SessionApplicationRole::Supporting,
                requirement: Some(ApplicationRequirement::Required),
                state: SessionApplicationState::Starting,
            },
            SessionApplicationSnapshot {
                application_id: simhub_id,
                name: "SimHub".to_owned(),
                role: SessionApplicationRole::Supporting,
                requirement: Some(ApplicationRequirement::Optional),
                state: SessionApplicationState::Pending,
            },
            SessionApplicationSnapshot {
                application_id: primary_sim_id,
                name: "Le Mans Ultimate".to_owned(),
                role: SessionApplicationRole::PrimarySim,
                requirement: None,
                state: SessionApplicationState::Pending,
            },
        ]
    );
    assert_eq!(
        snapshot.application_processes,
        vec![formation_lap_lib::ApplicationProcessSnapshot {
            application_id: crew_chief_id,
            status: ProcessStatus::Starting,
            ownership: Some(ProcessOwnership::SessionOwned),
            identity: Some(first_identity),
            output: None,
        }]
    );
}

#[test]
fn refresh_advances_the_saved_sequence_and_confirms_the_primary_sim_last() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let identities = [9_708, 10_220, 10_732].map(|pid| ProcessIdentity {
        pid,
        creation_time: format!("133822945{pid}00000"),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    });
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::from([Vec::new(), Vec::new(), Vec::new()]),
        launch_results: identities.iter().cloned().map(Ok).collect::<VecDeque<_>>(),
        ..ScriptedProcessRuntime::default()
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("empty Session storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Sprint".to_owned(),
            primary_sim_name: "Primary Sim".to_owned(),
        })
        .expect("fixture profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("fixture profile should be selected");
    profile.supporting_applications = vec![
        SupportingApplication {
            application: application("new-first", "First Support", &executable_path),
            requirement: ApplicationRequirement::Required,
            keep_running: false,
        },
        SupportingApplication {
            application: application("new-second", "Second Support", &executable_path),
            requirement: ApplicationRequirement::Optional,
            keep_running: false,
        },
    ];
    profile.primary_sim = application(&profile.primary_sim.id, "Primary Sim", &executable_path);
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("fixture profile should be configured");

    core.execute(AppCommand::StartSession {
        profile_id: profile_id.clone(),
    })
    .expect("Session should start");

    core.execute(AppCommand::RefreshProcesses)
        .expect("first Supporting Application should become ready");
    assert_eq!(
        core.snapshot()
            .session
            .applications
            .iter()
            .map(|application| application.state.clone())
            .collect::<Vec<_>>(),
        vec![
            SessionApplicationState::Running,
            SessionApplicationState::Starting,
            SessionApplicationState::Pending,
        ]
    );

    core.execute(AppCommand::RefreshProcesses)
        .expect("second Supporting Application should become ready");
    assert_eq!(
        core.snapshot()
            .session
            .applications
            .iter()
            .map(|application| application.state.clone())
            .collect::<Vec<_>>(),
        vec![
            SessionApplicationState::Running,
            SessionApplicationState::Running,
            SessionApplicationState::Starting,
        ]
    );

    core.execute(AppCommand::RefreshProcesses)
        .expect("Primary Sim should become ready last");
    let snapshot = core.snapshot();
    assert_eq!(snapshot.session.state, SessionState::Active);
    assert_eq!(
        snapshot
            .session
            .applications
            .iter()
            .map(|application| application.state.clone())
            .collect::<Vec<_>>(),
        vec![
            SessionApplicationState::Running,
            SessionApplicationState::Running,
            SessionApplicationState::Running,
        ]
    );
    assert_eq!(snapshot.session.active_profile_id, Some(profile_id));
}

#[test]
fn required_launch_failure_blocks_the_primary_sim_and_returns_to_idle() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::from([Vec::new()]),
        launch_results: VecDeque::from([Err(ProcessRuntimeError::new(
            "required fixture could not start",
        ))]),
        ..ScriptedProcessRuntime::default()
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("empty Session storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Required failure".to_owned(),
            primary_sim_name: "Primary Sim".to_owned(),
        })
        .expect("fixture profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("fixture profile should be selected");
    profile.supporting_applications = vec![SupportingApplication {
        application: application("new-required", "Required Support", &executable_path),
        requirement: ApplicationRequirement::Required,
        keep_running: false,
    }];
    profile.primary_sim = application(&profile.primary_sim.id, "Primary Sim", &executable_path);
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("fixture profile should be configured");
    let configured_profile = core
        .snapshot()
        .selected_profile
        .expect("configured profile should remain selected");
    let required_id = configured_profile.supporting_applications[0]
        .application
        .id
        .clone();
    let primary_id = configured_profile.primary_sim.id.clone();

    let outcome = core
        .execute(AppCommand::StartSession { profile_id })
        .expect("a launch failure is an observable Session outcome");

    assert_eq!(
        outcome,
        CommandOutcome::SessionStartFailed {
            application_id: required_id.clone(),
        }
    );
    let snapshot = core.snapshot();
    assert_eq!(snapshot.session.state, SessionState::Idle);
    assert_eq!(snapshot.session.active_profile_id, None);
    assert_eq!(
        snapshot
            .session
            .applications
            .iter()
            .map(|application| {
                (
                    application.application_id.clone(),
                    application.state.clone(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (required_id, SessionApplicationState::Failed),
            (primary_id, SessionApplicationState::Pending),
        ]
    );
    assert!(
        snapshot.application_processes.is_empty(),
        "the Primary Sim must not launch after a Required Application fails"
    );
}

#[test]
fn optional_launch_failure_is_recorded_and_the_primary_sim_still_starts() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let primary_identity = ProcessIdentity {
        pid: 11_244,
        creation_time: "133822945112440000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::from([Vec::new(), Vec::new()]),
        launch_results: VecDeque::from([
            Err(ProcessRuntimeError::new("optional fixture could not start")),
            Ok(primary_identity.clone()),
        ]),
        ..ScriptedProcessRuntime::default()
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("empty Session storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Optional failure".to_owned(),
            primary_sim_name: "Primary Sim".to_owned(),
        })
        .expect("fixture profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("fixture profile should be selected");
    profile.supporting_applications = vec![SupportingApplication {
        application: application("new-optional", "Optional Support", &executable_path),
        requirement: ApplicationRequirement::Optional,
        keep_running: false,
    }];
    profile.primary_sim = application(&profile.primary_sim.id, "Primary Sim", &executable_path);
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("fixture profile should be configured");
    let configured_profile = core
        .snapshot()
        .selected_profile
        .expect("configured profile should remain selected");
    let optional_id = configured_profile.supporting_applications[0]
        .application
        .id
        .clone();
    let primary_id = configured_profile.primary_sim.id.clone();

    let outcome = core
        .execute(AppCommand::StartSession {
            profile_id: profile_id.clone(),
        })
        .expect("Optional Application failure should not fail the Session command");

    assert_eq!(
        outcome,
        CommandOutcome::SessionStartRequested { profile_id }
    );
    let snapshot = core.snapshot();
    assert_eq!(snapshot.session.state, SessionState::Starting);
    assert_eq!(
        snapshot
            .session
            .applications
            .iter()
            .map(|application| {
                (
                    application.application_id.clone(),
                    application.state.clone(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (optional_id, SessionApplicationState::Failed),
            (primary_id.clone(), SessionApplicationState::Starting),
        ]
    );
    assert_eq!(
        snapshot.application_processes,
        vec![formation_lap_lib::ApplicationProcessSnapshot {
            application_id: primary_id,
            status: ProcessStatus::Starting,
            ownership: Some(ProcessOwnership::SessionOwned),
            identity: Some(primary_identity),
            output: None,
        }]
    );
}

#[test]
fn cancel_startup_stops_only_attempt_owned_processes_and_never_launches_the_primary_sim() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let pre_existing_identity = ProcessIdentity {
        pid: 11_756,
        creation_time: "133822945117560000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let attempt_owned_identity = ProcessIdentity {
        pid: 12_268,
        creation_time: "133822945122680000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::from([vec![pre_existing_identity.clone()], Vec::new()]),
        launch_results: VecDeque::from([Ok(attempt_owned_identity)]),
        graceful_stop_results: VecDeque::from([Ok(GracefulStopResult::Requested)]),
        wait_for_exit_results: VecDeque::from([Ok(true)]),
        ..ScriptedProcessRuntime::default()
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("empty Session storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Cancelled startup".to_owned(),
            primary_sim_name: "Primary Sim".to_owned(),
        })
        .expect("fixture profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("fixture profile should be selected");
    profile.supporting_applications = vec![
        SupportingApplication {
            application: application(
                "pre-existing-support",
                "Pre-existing Support",
                &executable_path,
            ),
            requirement: ApplicationRequirement::Required,
            keep_running: false,
        },
        SupportingApplication {
            application: application(
                "attempt-owned-support",
                "Attempt-owned Support",
                &executable_path,
            ),
            requirement: ApplicationRequirement::Required,
            keep_running: false,
        },
    ];
    profile.primary_sim = application(&profile.primary_sim.id, "Primary Sim", &executable_path);
    let primary_id = profile.primary_sim.id.clone();
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("fixture profile should be configured");
    let configured_profile = core
        .snapshot()
        .selected_profile
        .expect("configured profile should remain selected");
    let pre_existing_id = configured_profile.supporting_applications[0]
        .application
        .id
        .clone();
    let attempt_owned_id = configured_profile.supporting_applications[1]
        .application
        .id
        .clone();
    core.execute(AppCommand::StartSession { profile_id })
        .expect("Session should begin with the Pre-existing Process");
    core.execute(AppCommand::RefreshProcesses)
        .expect("second Supporting Application should start");

    let outcome = core
        .execute(AppCommand::CancelStartup)
        .expect("Starting should accept Cancel Startup");

    assert_eq!(outcome, CommandOutcome::SessionCancellationRequested);
    assert_eq!(core.snapshot().session.state, SessionState::Cancelling);

    core.execute(AppCommand::RefreshProcesses)
        .expect("attempt cleanup should finish");
    let snapshot = core.snapshot();
    assert_eq!(snapshot.session.state, SessionState::Idle);
    assert_eq!(snapshot.session.active_profile_id, None);
    assert_eq!(
        snapshot
            .session
            .applications
            .iter()
            .map(|application| {
                (
                    application.application_id.as_str(),
                    application.state.clone(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (pre_existing_id.as_str(), SessionApplicationState::Detached),
            (attempt_owned_id.as_str(), SessionApplicationState::Stopped),
            (primary_id.as_str(), SessionApplicationState::Pending),
        ]
    );
    let pre_existing = snapshot
        .application_processes
        .iter()
        .find(|process| process.application_id == pre_existing_id)
        .expect("Pre-existing Process should remain visible");
    assert_eq!(pre_existing.ownership, Some(ProcessOwnership::PreExisting));
    assert_eq!(pre_existing.identity, Some(pre_existing_identity));
    let attempt_owned = snapshot
        .application_processes
        .iter()
        .find(|process| process.application_id == attempt_owned_id)
        .expect("attempt-owned Process should remain as stopped history");
    assert_eq!(attempt_owned.status, ProcessStatus::Stopped);
    assert_eq!(attempt_owned.ownership, None);
    assert_eq!(attempt_owned.identity, None);
    assert!(
        snapshot
            .application_processes
            .iter()
            .all(|process| process.application_id != primary_id),
        "Cancel Startup must not launch the next entry"
    );
}

#[test]
fn close_session_stops_primary_then_owned_supports_in_reverse_and_detaches_preserved_processes() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let identities = [12_780, 13_292, 13_804, 14_316, 14_828].map(|pid| ProcessIdentity {
        pid,
        creation_time: format!("133822945{pid}00000"),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    });
    let stop_trace = Arc::new(Mutex::new(Vec::new()));
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::from([
            Vec::new(),
            vec![identities[1].clone()],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ]),
        launch_results: VecDeque::from([
            Ok(identities[0].clone()),
            Ok(identities[2].clone()),
            Ok(identities[3].clone()),
            Ok(identities[4].clone()),
        ]),
        graceful_stop_results: VecDeque::from([
            Ok(GracefulStopResult::Requested),
            Ok(GracefulStopResult::Requested),
            Ok(GracefulStopResult::Requested),
        ]),
        wait_for_exit_results: VecDeque::from([Ok(true), Ok(true), Ok(true)]),
        stop_trace: Some(Arc::clone(&stop_trace)),
        ..ScriptedProcessRuntime::default()
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("empty Session storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Orderly close".to_owned(),
            primary_sim_name: "Primary Sim".to_owned(),
        })
        .expect("fixture profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("fixture profile should be selected");
    profile.supporting_applications = vec![
        SupportingApplication {
            application: application("new-first-owned", "First owned", &executable_path),
            requirement: ApplicationRequirement::Required,
            keep_running: false,
        },
        SupportingApplication {
            application: application("new-pre-existing", "Pre-existing", &executable_path),
            requirement: ApplicationRequirement::Required,
            keep_running: false,
        },
        SupportingApplication {
            application: application("new-last-owned", "Last owned", &executable_path),
            requirement: ApplicationRequirement::Required,
            keep_running: false,
        },
        SupportingApplication {
            application: application("new-keep-running", "Keep running", &executable_path),
            requirement: ApplicationRequirement::Required,
            keep_running: true,
        },
    ];
    profile.primary_sim = application(&profile.primary_sim.id, "Primary Sim", &executable_path);
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("fixture profile should be configured");
    let configured_profile = core
        .snapshot()
        .selected_profile
        .expect("configured profile should remain selected");
    let first_owned_id = configured_profile.supporting_applications[0]
        .application
        .id
        .clone();
    let pre_existing_id = configured_profile.supporting_applications[1]
        .application
        .id
        .clone();
    let last_owned_id = configured_profile.supporting_applications[2]
        .application
        .id
        .clone();
    let keep_running_id = configured_profile.supporting_applications[3]
        .application
        .id
        .clone();
    let primary_id = configured_profile.primary_sim.id.clone();

    core.execute(AppCommand::StartSession {
        profile_id: profile_id.clone(),
    })
    .expect("Session should begin");
    for _ in 0..5 {
        core.execute(AppCommand::RefreshProcesses)
            .expect("ordered startup should advance");
    }
    assert_eq!(core.snapshot().session.state, SessionState::Active);

    let outcome = core
        .execute(AppCommand::CloseSession)
        .expect("Active should accept Close Session");

    assert_eq!(outcome, CommandOutcome::SessionCloseRequested);
    assert_eq!(core.snapshot().session.state, SessionState::Closing);
    assert!(matches!(
        core.execute(AppCommand::RestartApplication {
            profile_id: profile_id.clone(),
            application_id: primary_id.clone(),
            pre_existing_confirmed: false,
        }),
        Err(CoreError::InvalidSessionTransition {
            current: SessionState::Closing,
            command: "Restart Application",
        })
    ));

    core.execute(AppCommand::RefreshProcesses)
        .expect("eligible Session cleanup should finish");
    let snapshot = core.snapshot();
    assert_eq!(snapshot.session.state, SessionState::Idle);
    assert_eq!(snapshot.session.active_profile_id, None);
    assert_eq!(
        *stop_trace.lock().expect("stop trace should be readable"),
        vec![identities[4].pid, identities[2].pid, identities[0].pid],
        "the Primary Sim closes first, followed by eligible Supporting Applications in reverse order"
    );
    for application_id in [&primary_id, &last_owned_id, &first_owned_id] {
        let process = snapshot
            .application_processes
            .iter()
            .find(|process| process.application_id == *application_id)
            .expect("closed Process should remain as history");
        assert_eq!(process.status, ProcessStatus::Stopped);
        assert_eq!(process.ownership, None);
        assert_eq!(process.identity, None);
    }
    for application_id in [&pre_existing_id, &keep_running_id] {
        let process = snapshot
            .application_processes
            .iter()
            .find(|process| process.application_id == *application_id)
            .expect("preserved Process should remain visible");
        assert_eq!(process.status, ProcessStatus::RunningPreExisting);
        assert_eq!(process.ownership, Some(ProcessOwnership::PreExisting));
        assert!(process.identity.is_some());
        let session_application = snapshot
            .session
            .applications
            .iter()
            .find(|application| application.application_id == *application_id)
            .expect("preserved Process should remain on the Formation Rail");
        assert_eq!(session_application.state, SessionApplicationState::Detached);
    }
}

#[test]
fn unexpected_primary_sim_exit_begins_cleanup_exactly_once() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let support_identity = ProcessIdentity {
        pid: 15_340,
        creation_time: "133822945153400000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let primary_identity = ProcessIdentity {
        pid: 15_852,
        creation_time: "133822945158520000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let running = ProcessObservation::Running {
        responsiveness: formation_lap_lib::ProcessResponsiveness::NotApplicable,
    };
    let stop_trace = Arc::new(Mutex::new(Vec::new()));
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::from([Vec::new(), Vec::new()]),
        launch_results: VecDeque::from([
            Ok(support_identity.clone()),
            Ok(primary_identity.clone()),
        ]),
        observations_by_pid: BTreeMap::from([
            (
                support_identity.pid,
                VecDeque::from([running.clone(), running.clone(), running.clone(), running]),
            ),
            (
                primary_identity.pid,
                VecDeque::from([
                    ProcessObservation::Running {
                        responsiveness: formation_lap_lib::ProcessResponsiveness::NotApplicable,
                    },
                    ProcessObservation::Exited,
                ]),
            ),
        ]),
        graceful_stop_results: VecDeque::from([Ok(GracefulStopResult::Requested)]),
        wait_for_exit_results: VecDeque::from([Ok(true)]),
        stop_trace: Some(Arc::clone(&stop_trace)),
        ..ScriptedProcessRuntime::default()
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("empty Session storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Unexpected exit".to_owned(),
            primary_sim_name: "Primary Sim".to_owned(),
        })
        .expect("fixture profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("fixture profile should be selected");
    profile.supporting_applications = vec![SupportingApplication {
        application: application("new-support", "Support", &executable_path),
        requirement: ApplicationRequirement::Required,
        keep_running: false,
    }];
    profile.primary_sim = application(&profile.primary_sim.id, "Primary Sim", &executable_path);
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("fixture profile should be configured");
    core.execute(AppCommand::StartSession { profile_id })
        .expect("Session should begin");
    core.execute(AppCommand::RefreshProcesses)
        .expect("Primary Sim should start after the support");
    core.execute(AppCommand::RefreshProcesses)
        .expect("Primary Sim should become active");
    assert_eq!(core.snapshot().session.state, SessionState::Active);

    core.execute(AppCommand::RefreshProcesses)
        .expect("Primary Sim exit should be observed");

    assert_eq!(core.snapshot().session.state, SessionState::Closing);
    assert!(
        stop_trace
            .lock()
            .expect("stop trace should be readable")
            .is_empty(),
        "the exit observation should transition once before cleanup advances"
    );

    core.execute(AppCommand::RefreshProcesses)
        .expect("the remaining support should be cleaned up");
    core.execute(AppCommand::RefreshProcesses)
        .expect("an Idle refresh should be harmless");
    assert_eq!(core.snapshot().session.state, SessionState::Idle);
    assert_eq!(
        *stop_trace.lock().expect("stop trace should be readable"),
        vec![support_identity.pid],
        "the remaining Process must be cleaned exactly once"
    );
}

#[test]
fn post_start_delay_holds_the_sequence_until_the_entry_is_ready() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let support_identity = ProcessIdentity {
        pid: 16_364,
        creation_time: "133822945163640000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let primary_identity = ProcessIdentity {
        pid: 16_876,
        creation_time: "133822945168760000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::from([Vec::new(), Vec::new()]),
        launch_results: VecDeque::from([Ok(support_identity), Ok(primary_identity.clone())]),
        ..ScriptedProcessRuntime::default()
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("empty Session storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Post-start delay".to_owned(),
            primary_sim_name: "Primary Sim".to_owned(),
        })
        .expect("fixture profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("fixture profile should be selected");
    let mut delayed_support = application("new-delayed", "Delayed Support", &executable_path);
    delayed_support.launch_recipe.post_start_delay_milliseconds = 40;
    profile.supporting_applications = vec![SupportingApplication {
        application: delayed_support,
        requirement: ApplicationRequirement::Required,
        keep_running: false,
    }];
    profile.primary_sim = application(&profile.primary_sim.id, "Primary Sim", &executable_path);
    let primary_id = profile.primary_sim.id.clone();
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("fixture profile should be configured");
    core.execute(AppCommand::StartSession { profile_id })
        .expect("Session should begin");

    core.execute(AppCommand::RefreshProcesses)
        .expect("the support Process should appear");

    let snapshot = core.snapshot();
    assert_eq!(
        snapshot.session.applications[0].state,
        SessionApplicationState::Starting
    );
    assert!(
        snapshot
            .application_processes
            .iter()
            .all(|process| process.application_id != primary_id),
        "the Primary Sim must wait for the post-start delay"
    );

    std::thread::sleep(Duration::from_millis(50));
    core.execute(AppCommand::RefreshProcesses)
        .expect("the elapsed delay should release the sequence");
    let snapshot = core.snapshot();
    assert_eq!(
        snapshot.session.applications[0].state,
        SessionApplicationState::Running
    );
    assert_eq!(
        snapshot.session.applications[1].state,
        SessionApplicationState::Starting
    );
    assert_eq!(
        snapshot
            .application_processes
            .iter()
            .find(|process| process.application_id == primary_id)
            .and_then(|process| process.identity.clone()),
        Some(primary_identity)
    );
}

#[test]
fn optional_startup_timeout_is_recorded_and_the_sequence_continues() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let timed_out_identity = ProcessIdentity {
        pid: 17_388,
        creation_time: "133822945173880000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let primary_identity = ProcessIdentity {
        pid: 17_900,
        creation_time: "133822945179000000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::from([Vec::new(), Vec::new()]),
        launch_results: VecDeque::from([
            Ok(timed_out_identity.clone()),
            Ok(primary_identity.clone()),
        ]),
        ..ScriptedProcessRuntime::default()
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("empty Session storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Optional timeout".to_owned(),
            primary_sim_name: "Primary Sim".to_owned(),
        })
        .expect("fixture profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("fixture profile should be selected");
    let mut timed_out_support = application("new-timeout", "Timed-out Support", &executable_path);
    timed_out_support.launch_recipe.startup_timeout_seconds = 0;
    profile.supporting_applications = vec![SupportingApplication {
        application: timed_out_support,
        requirement: ApplicationRequirement::Optional,
        keep_running: false,
    }];
    profile.primary_sim = application(&profile.primary_sim.id, "Primary Sim", &executable_path);
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("fixture profile should be configured");
    core.execute(AppCommand::StartSession { profile_id })
        .expect("Session should begin");

    core.execute(AppCommand::RefreshProcesses)
        .expect("Optional timeout should not fail the refresh command");

    let snapshot = core.snapshot();
    assert_eq!(snapshot.session.state, SessionState::Starting);
    assert_eq!(
        snapshot
            .session
            .applications
            .iter()
            .map(|application| application.state.clone())
            .collect::<Vec<_>>(),
        vec![
            SessionApplicationState::Failed,
            SessionApplicationState::Starting,
        ]
    );
    assert_eq!(
        snapshot
            .application_processes
            .iter()
            .find(|process| process.identity.as_ref() == Some(&timed_out_identity))
            .map(|process| process.status.clone()),
        Some(ProcessStatus::Failed)
    );
    assert!(
        snapshot
            .application_processes
            .iter()
            .any(|process| process.identity.as_ref() == Some(&primary_identity)),
        "the Primary Sim should launch after an Optional timeout"
    );
}

#[test]
fn a_starting_session_rejects_a_competing_start_and_locks_its_profile() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let primary_identity = ProcessIdentity {
        pid: 18_412,
        creation_time: "133822945184120000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::from([Vec::new()]),
        launch_results: VecDeque::from([Ok(primary_identity)]),
        ..ScriptedProcessRuntime::default()
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("empty Session storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Locked profile".to_owned(),
            primary_sim_name: "Primary Sim".to_owned(),
        })
        .expect("fixture profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("fixture profile should be selected");
    profile.primary_sim = application(&profile.primary_sim.id, "Primary Sim", &executable_path);
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("fixture profile should be configured");
    core.execute(AppCommand::StartSession {
        profile_id: profile_id.clone(),
    })
    .expect("Idle should accept the first Start Session");

    let competing_start = core
        .execute(AppCommand::StartSession {
            profile_id: profile_id.clone(),
        })
        .expect_err("Starting must reject a competing Start Session");
    assert!(matches!(
        competing_start,
        CoreError::InvalidSessionTransition {
            current: SessionState::Starting,
            command: "Start Session",
        }
    ));

    let mut edited_profile = core
        .snapshot()
        .selected_profile
        .expect("active profile should remain visible");
    edited_profile.name = "Unsafe edit".to_owned();
    let edit = core
        .execute(AppCommand::SaveProfile {
            profile: Box::new(edited_profile),
        })
        .expect_err("the active Racing Profile must be locked");
    assert!(matches!(
        edit,
        CoreError::InvalidSessionTransition {
            current: SessionState::Starting,
            command: "Edit Active Profile",
        }
    ));
    assert_eq!(
        core.snapshot()
            .selected_profile
            .expect("active profile should remain visible")
            .name,
        "Locked profile"
    );
}

#[test]
fn active_session_events_stay_quiet_until_the_post_session_summary() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let support_identity = ProcessIdentity {
        pid: 18_924,
        creation_time: "133822945189240000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let primary_identity = ProcessIdentity {
        pid: 19_436,
        creation_time: "133822945194360000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let running = ProcessObservation::Running {
        responsiveness: formation_lap_lib::ProcessResponsiveness::NotApplicable,
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::from([Vec::new(), Vec::new()]),
        launch_results: VecDeque::from([
            Ok(support_identity.clone()),
            Ok(primary_identity.clone()),
        ]),
        observations_by_pid: BTreeMap::from([
            (
                support_identity.pid,
                VecDeque::from([running.clone(), running.clone(), ProcessObservation::Exited]),
            ),
            (
                primary_identity.pid,
                VecDeque::from([running.clone(), running, ProcessObservation::Exited]),
            ),
        ]),
        ..ScriptedProcessRuntime::default()
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("empty Session storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Quiet race".to_owned(),
            primary_sim_name: "Primary Sim".to_owned(),
        })
        .expect("fixture profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("fixture profile should be selected");
    profile.supporting_applications = vec![SupportingApplication {
        application: application("new-support", "Race Support", &executable_path),
        requirement: ApplicationRequirement::Optional,
        keep_running: false,
    }];
    profile.primary_sim = application(&profile.primary_sim.id, "Primary Sim", &executable_path);
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("fixture profile should be configured");
    let configured_profile = core
        .snapshot()
        .selected_profile
        .expect("configured profile should remain selected");
    let support_id = configured_profile.supporting_applications[0]
        .application
        .id
        .clone();
    let primary_id = configured_profile.primary_sim.id.clone();
    core.execute(AppCommand::StartSession {
        profile_id: profile_id.clone(),
    })
    .expect("Session should begin");
    core.execute(AppCommand::RefreshProcesses)
        .expect("Primary Sim should start");
    core.execute(AppCommand::RefreshProcesses)
        .expect("Session should become Active");
    assert_eq!(core.snapshot().session.state, SessionState::Active);

    core.execute(AppCommand::RefreshProcesses)
        .expect("support exit should be recorded quietly");

    let active_snapshot = core.snapshot();
    assert_eq!(active_snapshot.session.state, SessionState::Active);
    assert_eq!(
        active_snapshot.session.summary, None,
        "the Active Session must not surface an unsolicited summary"
    );

    core.execute(AppCommand::RefreshProcesses)
        .expect("Primary Sim exit should begin close");
    core.execute(AppCommand::RefreshProcesses)
        .expect("close should complete");
    let idle_snapshot = core.snapshot();
    assert_eq!(idle_snapshot.session.state, SessionState::Idle);
    assert_eq!(
        idle_snapshot.session.summary,
        Some(SessionSummary {
            profile_id,
            events: vec![
                SessionEvent {
                    application_id: support_id,
                    name: "Race Support".to_owned(),
                    kind: SessionEventKind::ApplicationExited,
                },
                SessionEvent {
                    application_id: primary_id,
                    name: "Primary Sim".to_owned(),
                    kind: SessionEventKind::ApplicationExited,
                }
            ],
        })
    );
}

#[test]
fn a_verified_journal_offers_recovery_without_resuming_until_accepted() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let primary_identity = ProcessIdentity {
        pid: 19_948,
        creation_time: "133822945199480000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let profile_id;
    {
        let runtime = ScriptedProcessRuntime {
            matching_processes: VecDeque::from([Vec::new()]),
            launch_results: VecDeque::from([Ok(primary_identity.clone())]),
            ..ScriptedProcessRuntime::default()
        };
        let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
            .expect("empty Session storage should open");
        profile_id = match core
            .execute(AppCommand::CreateProfile {
                name: "Recoverable".to_owned(),
                primary_sim_name: "Primary Sim".to_owned(),
            })
            .expect("fixture profile should be created")
        {
            CommandOutcome::ProfileCreated { profile_id } => profile_id,
            other => panic!("expected profile creation, got {other:?}"),
        };
        let mut profile = core
            .snapshot()
            .selected_profile
            .expect("fixture profile should be selected");
        profile.primary_sim = application(&profile.primary_sim.id, "Primary Sim", &executable_path);
        core.execute(AppCommand::SaveProfile {
            profile: Box::new(profile),
        })
        .expect("fixture profile should be configured");
        core.execute(AppCommand::StartSession {
            profile_id: profile_id.clone(),
        })
        .expect("Session should begin");
        core.execute(AppCommand::RefreshProcesses)
            .expect("Primary Sim should become Active");
        assert_eq!(core.snapshot().session.state, SessionState::Active);
        assert!(
            storage.path().join("active-session.json").is_file(),
            "ownership must be journaled before the launcher can exit"
        );
    }

    let recovery_runtime = ScriptedProcessRuntime::default();
    let mut recovered = FormationLapCore::open_with_runtime(storage.path(), recovery_runtime)
        .expect("verified journal should reopen");
    let recovery_snapshot = recovered.snapshot();
    assert_eq!(
        recovery_snapshot.session.state,
        SessionState::RecoveryAvailable
    );
    assert_eq!(
        recovery_snapshot.session.active_profile_id,
        Some(profile_id)
    );
    assert_eq!(
        recovery_snapshot.application_processes[0].identity,
        Some(primary_identity.clone())
    );

    let outcome = recovered
        .execute(AppCommand::AcceptRecovery)
        .expect("Recovery Offer should resume monitoring only after acceptance");
    assert_eq!(outcome, CommandOutcome::RecoveryAccepted);
    assert_eq!(recovered.snapshot().session.state, SessionState::Active);
    drop(recovered);

    let mut dismissed_core =
        FormationLapCore::open_with_runtime(storage.path(), ScriptedProcessRuntime::default())
            .expect("accepted Session should remain recoverable after another launcher exit");
    let outcome = dismissed_core
        .execute(AppCommand::DismissRecovery)
        .expect("Recovery Offer should be dismissible");
    assert_eq!(outcome, CommandOutcome::RecoveryDismissed);
    let dismissed = dismissed_core.snapshot();
    assert_eq!(dismissed.session.state, SessionState::Idle);
    assert_eq!(
        dismissed.application_processes[0].ownership,
        Some(ProcessOwnership::PreExisting),
        "dismissal must leave the verified Process untouched and unmanaged"
    );
    assert_eq!(
        dismissed.application_processes[0].identity,
        Some(primary_identity)
    );
    assert!(
        !storage.path().join("active-session.json").exists(),
        "dismissal should remove the stale Recovery Offer"
    );
}

#[test]
fn a_late_required_failure_cleans_the_processes_started_by_that_attempt() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let first_identity = ProcessIdentity {
        pid: 20_460,
        creation_time: "133822945204600000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::from([Vec::new(), Vec::new()]),
        launch_results: VecDeque::from([
            Ok(first_identity.clone()),
            Err(ProcessRuntimeError::new(
                "the later Required Application failed",
            )),
        ]),
        graceful_stop_results: VecDeque::from([Ok(GracefulStopResult::Requested)]),
        wait_for_exit_results: VecDeque::from([Ok(true)]),
        ..ScriptedProcessRuntime::default()
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("empty Session storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Late failure".to_owned(),
            primary_sim_name: "Primary Sim".to_owned(),
        })
        .expect("fixture profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("fixture profile should be selected");
    profile.supporting_applications = vec![
        SupportingApplication {
            application: application("new-first", "First Support", &executable_path),
            requirement: ApplicationRequirement::Required,
            keep_running: false,
        },
        SupportingApplication {
            application: application("new-failing", "Failing Support", &executable_path),
            requirement: ApplicationRequirement::Required,
            keep_running: false,
        },
    ];
    profile.primary_sim = application(&profile.primary_sim.id, "Primary Sim", &executable_path);
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("fixture profile should be configured");
    let first_id = core
        .snapshot()
        .selected_profile
        .expect("configured profile should remain selected")
        .supporting_applications[0]
        .application
        .id
        .clone();
    core.execute(AppCommand::StartSession { profile_id })
        .expect("Session should begin");

    core.execute(AppCommand::RefreshProcesses)
        .expect("Required failure should become a completed Session outcome");

    let snapshot = core.snapshot();
    assert_eq!(snapshot.session.state, SessionState::Idle);
    let first_process = snapshot
        .application_processes
        .iter()
        .find(|process| process.application_id == first_id)
        .expect("the first Process should remain in stopped history");
    assert_eq!(first_process.status, ProcessStatus::Stopped);
    assert_eq!(first_process.ownership, None);
    assert_eq!(first_process.identity, None);
    assert!(
        snapshot.application_processes.iter().all(
            |process| process.application_id != snapshot.session.applications[2].application_id
        ),
        "the Primary Sim must remain unlaunched"
    );
}

#[test]
fn the_native_command_loop_serializes_competing_start_requests() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let primary_identity = ProcessIdentity {
        pid: 20_972,
        creation_time: "133822945209720000".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let runtime = ScriptedProcessRuntime {
        matching_processes: VecDeque::from([Vec::new()]),
        launch_results: VecDeque::from([Ok(primary_identity)]),
        ..ScriptedProcessRuntime::default()
    };
    let commands = NativeCommandHost::open_with_runtime(storage.path(), runtime)
        .expect("native command host should open");
    let snapshot = commands
        .create_profile(CreateProfilePayload {
            name: "Serialized".to_owned(),
            primary_sim_name: "Primary Sim".to_owned(),
        })
        .expect("fixture profile should be created");
    let mut profile = snapshot
        .selected_profile
        .expect("fixture profile should be selected");
    let profile_id = profile.id.clone();
    profile.primary_sim = application(&profile.primary_sim.id, "Primary Sim", &executable_path);
    commands
        .save_profile(SaveProfilePayload { profile })
        .expect("fixture profile should be configured");
    let barrier = Arc::new(Barrier::new(3));
    let mut requests = Vec::new();
    for _ in 0..2 {
        let commands = commands.clone();
        let barrier = Arc::clone(&barrier);
        let profile_id = profile_id.clone();
        requests.push(std::thread::spawn(move || {
            barrier.wait();
            commands.start_session(ProfileIdPayload { profile_id })
        }));
    }
    barrier.wait();
    let results = requests
        .into_iter()
        .map(|request| request.join().expect("Start request thread should finish"))
        .collect::<Vec<_>>();

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter_map(|result| result.as_ref().err())
            .filter(|error| error.code == "invalid_session_transition")
            .count(),
        1
    );
    let snapshot = commands
        .get_app_snapshot()
        .expect("serialized snapshot should remain available");
    assert_eq!(snapshot.session.state, SessionState::Starting);
    assert_eq!(snapshot.session.active_profile_id, Some(profile_id));
    assert_eq!(
        snapshot
            .application_processes
            .iter()
            .filter(|process| process.ownership == Some(ProcessOwnership::SessionOwned))
            .count(),
        1,
        "competing requests must not create two Sessions"
    );
}

#[test]
fn idle_rejects_every_session_action_that_requires_an_existing_session() {
    let storage = TempStorage::new();
    let mut core =
        FormationLapCore::open_with_runtime(storage.path(), ScriptedProcessRuntime::default())
            .expect("empty Session storage should open");

    for (command, expected_name) in [
        (AppCommand::CancelStartup, "Cancel Startup"),
        (AppCommand::CloseSession, "Close Session"),
        (AppCommand::AcceptRecovery, "Resume Recovery"),
        (AppCommand::DismissRecovery, "Dismiss Recovery"),
    ] {
        match core.execute(command) {
            Err(CoreError::InvalidSessionTransition {
                current: SessionState::Idle,
                command,
            }) => assert_eq!(command, expected_name),
            other => panic!("expected Idle to reject {expected_name}, got {other:?}"),
        }
    }
}
