use formation_lap_lib::{
    AppCommand, CommandOutcome, ConsoleVisibility, CoreError, FormationLapCore, GracefulStopResult,
    LaunchRecipe, LaunchSource, ProcessIdentity, ProcessObservation, ProcessOutput, ProcessRuntime,
    ProcessRuntimeError, QuitAction, QuitDisposition, RacingProfile, SessionState,
    ShutdownStrategy, ThemePreference, WindowCloseAction,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::{
        Arc,
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
            "formation-lap-desktop-test-{}-{timestamp}-{unique}",
            process::id()
        ));
        fs::create_dir_all(&path).expect("temporary desktop storage should be created");
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

struct RunningProcessRuntime {
    next_identity: Option<ProcessIdentity>,
    stop_requests: Arc<AtomicU64>,
}

#[derive(Default)]
struct RuntimeTrace {
    launches: AtomicU64,
    observations: AtomicU64,
    stops: AtomicU64,
}

struct RecoveryProcessRuntime {
    observation: ProcessObservation,
    trace: Arc<RuntimeTrace>,
}

impl ProcessRuntime for RecoveryProcessRuntime {
    fn matching_processes(
        &mut self,
        _recipe: &LaunchRecipe,
    ) -> Result<Vec<ProcessIdentity>, ProcessRuntimeError> {
        Ok(Vec::new())
    }

    fn launch(&mut self, _recipe: &LaunchRecipe) -> Result<ProcessIdentity, ProcessRuntimeError> {
        self.trace.launches.fetch_add(1, Ordering::Relaxed);
        Err(ProcessRuntimeError::new(
            "Recovery verification must never launch",
        ))
    }

    fn observe(
        &mut self,
        _identity: &ProcessIdentity,
    ) -> Result<ProcessObservation, ProcessRuntimeError> {
        self.trace.observations.fetch_add(1, Ordering::Relaxed);
        Ok(self.observation.clone())
    }

    fn request_graceful_stop(
        &mut self,
        _identity: &ProcessIdentity,
        _strategy: &ShutdownStrategy,
    ) -> Result<GracefulStopResult, ProcessRuntimeError> {
        self.trace.stops.fetch_add(1, Ordering::Relaxed);
        Ok(GracefulStopResult::Requested)
    }

    fn wait_for_exit(
        &mut self,
        _identity: &ProcessIdentity,
        _timeout: Duration,
    ) -> Result<bool, ProcessRuntimeError> {
        Ok(false)
    }

    fn force_stop(&mut self, _identity: &ProcessIdentity) -> Result<(), ProcessRuntimeError> {
        self.trace.stops.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn read_output(
        &mut self,
        _identity: &ProcessIdentity,
    ) -> Result<ProcessOutput, ProcessRuntimeError> {
        Ok(ProcessOutput::default())
    }
}

impl ProcessRuntime for RunningProcessRuntime {
    fn matching_processes(
        &mut self,
        _recipe: &LaunchRecipe,
    ) -> Result<Vec<ProcessIdentity>, ProcessRuntimeError> {
        Ok(Vec::new())
    }

    fn launch(&mut self, _recipe: &LaunchRecipe) -> Result<ProcessIdentity, ProcessRuntimeError> {
        self.next_identity
            .take()
            .ok_or_else(|| ProcessRuntimeError::new("the launch fixture is exhausted"))
    }

    fn observe(
        &mut self,
        _identity: &ProcessIdentity,
    ) -> Result<ProcessObservation, ProcessRuntimeError> {
        Ok(ProcessObservation::Running {
            responsiveness: formation_lap_lib::ProcessResponsiveness::Responsive,
        })
    }

    fn request_graceful_stop(
        &mut self,
        _identity: &ProcessIdentity,
        _strategy: &ShutdownStrategy,
    ) -> Result<GracefulStopResult, ProcessRuntimeError> {
        self.stop_requests.fetch_add(1, Ordering::Relaxed);
        Ok(GracefulStopResult::Requested)
    }

    fn wait_for_exit(
        &mut self,
        _identity: &ProcessIdentity,
        _timeout: Duration,
    ) -> Result<bool, ProcessRuntimeError> {
        Ok(true)
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

fn configure_profile(mut profile: RacingProfile, executable_path: &Path) -> RacingProfile {
    profile.primary_sim.launch_recipe = LaunchRecipe {
        source: LaunchSource::DirectExecutable {
            executable_path: executable_path.to_string_lossy().into_owned(),
        },
        arguments: Vec::new(),
        working_directory: executable_path
            .parent()
            .map(|parent| parent.to_string_lossy().into_owned()),
        monitored_process: None,
        monitored_executable_path: None,
        console_visibility: ConsoleVisibility::Hidden,
        elevated: false,
        startup_timeout_seconds: 30,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::CloseWindows,
    };
    profile.primary_sim.path_needs_repair = false;
    profile
}

#[test]
fn native_window_close_exits_when_idle_and_hides_while_monitoring_a_session() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable should be available")
        .canonicalize()
        .expect("test executable should canonicalize");
    let identity = ProcessIdentity {
        pid: 42,
        creation_time: "100".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let runtime = RunningProcessRuntime {
        next_identity: Some(identity),
        stop_requests: Arc::new(AtomicU64::new(0)),
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("desktop test core should open");

    assert_eq!(
        core.execute(AppCommand::RequestWindowClose)
            .expect("idle window close should be decided"),
        CommandOutcome::WindowCloseRequested {
            action: WindowCloseAction::Exit
        }
    );

    let CommandOutcome::ProfileCreated { profile_id } = core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Race night".to_owned(),
                "Primary Sim".to_owned(),
            )),
        })
        .expect("profile should be created")
    else {
        panic!("create should return the profile ID");
    };
    let profile = configure_profile(
        core.snapshot()
            .selected_profile
            .expect("created profile should be selected"),
        &executable_path,
    );
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("profile should save");
    core.execute(AppCommand::ApproveProfile {
        profile_id: profile_id.clone(),
        configuration_reviewed: true,
        approved_privileged_application_ids: Vec::new(),
    })
    .expect("fixture configuration should be approved");
    core.execute(AppCommand::StartSession { profile_id })
        .expect("Session should start");
    core.execute(AppCommand::RefreshProcesses)
        .expect("Session should become Active");

    assert_eq!(
        core.execute(AppCommand::RequestWindowClose)
            .expect("active window close should be decided"),
        CommandOutcome::WindowCloseRequested {
            action: WindowCloseAction::HideToTray
        }
    );
}

fn active_core(storage: &TempStorage, stop_requests: Arc<AtomicU64>) -> FormationLapCore {
    let executable_path = std::env::current_exe()
        .expect("test executable should be available")
        .canonicalize()
        .expect("test executable should canonicalize");
    let identity = ProcessIdentity {
        pid: 84,
        creation_time: "200".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let runtime = RunningProcessRuntime {
        next_identity: Some(identity),
        stop_requests,
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("desktop test core should open");
    let CommandOutcome::ProfileCreated { profile_id } = core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Race night".to_owned(),
                "Primary Sim".to_owned(),
            )),
        })
        .expect("profile should be created")
    else {
        panic!("create should return the profile ID");
    };
    let profile = configure_profile(
        core.snapshot()
            .selected_profile
            .expect("created profile should be selected"),
        &executable_path,
    );
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("profile should save");
    core.execute(AppCommand::ApproveProfile {
        profile_id: profile_id.clone(),
        configuration_reviewed: true,
        approved_privileged_application_ids: Vec::new(),
    })
    .expect("fixture configuration should be approved");
    core.execute(AppCommand::StartSession { profile_id })
        .expect("Session should start");
    core.execute(AppCommand::RefreshProcesses)
        .expect("Session should become Active");
    assert_eq!(core.snapshot().session.state, SessionState::Active);
    core
}

#[test]
fn explicit_quit_closes_the_session_or_detaches_running_applications_by_choice() {
    let close_storage = TempStorage::new();
    let close_requests = Arc::new(AtomicU64::new(0));
    let mut close_core = active_core(&close_storage, Arc::clone(&close_requests));

    assert_eq!(
        close_core
            .execute(AppCommand::RequestQuit {
                disposition: QuitDisposition::CloseSession,
            })
            .expect("Quit with cleanup should begin closing"),
        CommandOutcome::QuitRequested {
            action: QuitAction::WaitForSessionClose
        }
    );
    assert_eq!(close_core.snapshot().session.state, SessionState::Closing);
    close_core
        .execute(AppCommand::RefreshProcesses)
        .expect("cleanup should complete");
    assert_eq!(close_core.snapshot().session.state, SessionState::Idle);
    assert_eq!(close_requests.load(Ordering::Relaxed), 1);

    let leave_storage = TempStorage::new();
    let leave_requests = Arc::new(AtomicU64::new(0));
    let mut leave_core = active_core(&leave_storage, Arc::clone(&leave_requests));

    assert_eq!(
        leave_core
            .execute(AppCommand::RequestQuit {
                disposition: QuitDisposition::LeaveApplicationsRunning,
            })
            .expect("Quit should detach applications only by explicit choice"),
        CommandOutcome::QuitRequested {
            action: QuitAction::ExitNow
        }
    );
    assert_eq!(leave_core.snapshot().session.state, SessionState::Idle);
    assert_eq!(leave_requests.load(Ordering::Relaxed), 0);

    let reopened = FormationLapCore::open_with_runtime(
        leave_storage.path(),
        RunningProcessRuntime {
            next_identity: None,
            stop_requests: Arc::new(AtomicU64::new(0)),
        },
    )
    .expect("explicitly detached Session should reopen without Recovery Offer");
    assert_eq!(reopened.snapshot().session.state, SessionState::Idle);
}

#[test]
fn desktop_settings_default_safe_and_persist_with_a_bounded_backup() {
    let storage = TempStorage::new();
    let runtime = RunningProcessRuntime {
        next_identity: None,
        stop_requests: Arc::new(AtomicU64::new(0)),
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("desktop test core should open");

    assert!(!core.snapshot().settings.start_with_windows);
    assert_eq!(core.snapshot().settings.theme, ThemePreference::System);
    assert!(!core.snapshot().settings.reduce_motion);

    let mut settings = core.snapshot().settings;
    settings.start_with_windows = true;
    settings.theme = ThemePreference::Dark;
    settings.reduce_motion = true;
    core.execute(AppCommand::UpdateSettings {
        settings: settings.clone(),
    })
    .expect("desktop settings should update");

    let mut second_settings = settings.clone();
    second_settings.theme = ThemePreference::Light;
    core.execute(AppCommand::UpdateSettings {
        settings: second_settings.clone(),
    })
    .expect("a second settings update should retain a backup");

    assert!(
        storage
            .path()
            .join("backups")
            .join("settings.json")
            .is_file(),
        "the previous valid settings document should be retained"
    );

    let reopened = FormationLapCore::open_with_runtime(
        storage.path(),
        RunningProcessRuntime {
            next_identity: None,
            stop_requests: Arc::new(AtomicU64::new(0)),
        },
    )
    .expect("saved settings should reopen");
    assert_eq!(reopened.snapshot().settings, second_settings);
}

#[test]
fn corrupt_live_settings_recovers_from_a_valid_backup_without_growing_artifacts() {
    let storage = TempStorage::new();
    let mut core = FormationLapCore::open_with_runtime(
        storage.path(),
        RunningProcessRuntime {
            next_identity: None,
            stop_requests: Arc::new(AtomicU64::new(0)),
        },
    )
    .expect("settings fixture should open");
    let mut backup_settings = core.snapshot().settings;
    backup_settings.theme = ThemePreference::Dark;
    core.execute(AppCommand::UpdateSettings {
        settings: backup_settings.clone(),
    })
    .expect("first settings write should succeed");
    let mut newer_settings = backup_settings.clone();
    newer_settings.theme = ThemePreference::Light;
    core.execute(AppCommand::UpdateSettings {
        settings: newer_settings,
    })
    .expect("second settings write should retain the first as backup");
    drop(core);

    let settings_path = storage.path().join("settings.json");
    fs::write(&settings_path, b"{ truncated settings")
        .expect("fixture should corrupt live settings");
    fs::write(
        settings_path.with_extension("json.recovery.tmp"),
        b"interrupted restore copy",
    )
    .expect("fixture should simulate an interrupted restore");

    let reopened = FormationLapCore::open_with_runtime(
        storage.path(),
        RunningProcessRuntime {
            next_identity: None,
            stop_requests: Arc::new(AtomicU64::new(0)),
        },
    )
    .expect("a valid settings backup should recover corrupt live settings");
    assert_eq!(reopened.snapshot().settings, backup_settings);
    assert!(
        serde_json::from_slice::<serde_json::Value>(
            &fs::read(&settings_path).expect("recovered live settings should remain readable"),
        )
        .is_ok()
    );
    assert!(
        storage
            .path()
            .join("backups")
            .join("settings.json")
            .is_file()
    );
    assert!(
        serde_json::from_slice::<serde_json::Value>(
            &fs::read(storage.path().join("backups").join("settings.json"))
                .expect("valid backup should survive interrupted restore"),
        )
        .is_ok()
    );
    let corrupt_artifacts = fs::read_dir(storage.path().join("backups"))
        .expect("backup directory should be readable")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".corrupt."))
        .count();
    assert_eq!(
        corrupt_artifacts, 1,
        "the first deterministic corrupt slot should be used"
    );
    assert!(
        corrupt_artifacts <= 2,
        "corrupt recovery artifacts must stay bounded"
    );
    let artifacts_before = fs::read_dir(storage.path().join("backups"))
        .expect("backup directory should be readable")
        .count();
    drop(reopened);
    FormationLapCore::open_with_runtime(
        storage.path(),
        RunningProcessRuntime {
            next_identity: None,
            stop_requests: Arc::new(AtomicU64::new(0)),
        },
    )
    .expect("reopening recovered settings should remain idempotent");
    assert_eq!(
        fs::read_dir(storage.path().join("backups"))
            .expect("backup directory should remain readable")
            .count(),
        artifacts_before,
        "recovery artifacts must be bounded across repeated startup"
    );
}

#[test]
fn invalid_live_and_backup_settings_are_preserved_and_reported() {
    let storage = TempStorage::new();
    let mut core = FormationLapCore::open_with_runtime(
        storage.path(),
        RunningProcessRuntime {
            next_identity: None,
            stop_requests: Arc::new(AtomicU64::new(0)),
        },
    )
    .expect("settings fixture should open");
    let settings = core.snapshot().settings;
    core.execute(AppCommand::UpdateSettings { settings })
        .expect("first settings write should succeed");
    core.execute(AppCommand::UpdateSettings {
        settings: core.snapshot().settings,
    })
    .expect("second settings write should retain a backup");
    drop(core);

    let live = storage.path().join("settings.json");
    let backup = storage.path().join("backups").join("settings.json");
    let corrupt_live = b"not settings".to_vec();
    let corrupt_backup = b"not backup settings".to_vec();
    fs::write(&live, &corrupt_live).expect("fixture should corrupt live settings");
    fs::write(&backup, &corrupt_backup).expect("fixture should corrupt settings backup");

    assert!(matches!(
        FormationLapCore::open_with_runtime(
            storage.path(),
            RunningProcessRuntime {
                next_identity: None,
                stop_requests: Arc::new(AtomicU64::new(0)),
            },
        ),
        Err(CoreError::InvalidSettingsDocument(_))
    ));
    assert_eq!(
        fs::read(&live).expect("live corrupt bytes should remain"),
        corrupt_live
    );
    assert_eq!(
        fs::read(&backup).expect("backup corrupt bytes should remain"),
        corrupt_backup
    );
}

#[test]
fn newer_live_settings_never_fall_back_to_an_older_backup() {
    let storage = TempStorage::new();
    let mut core = FormationLapCore::open_with_runtime(
        storage.path(),
        RunningProcessRuntime {
            next_identity: None,
            stop_requests: Arc::new(AtomicU64::new(0)),
        },
    )
    .expect("settings fixture should open");
    let mut settings = core.snapshot().settings;
    settings.theme = ThemePreference::Dark;
    core.execute(AppCommand::UpdateSettings { settings })
        .expect("first settings write should succeed");
    core.execute(AppCommand::UpdateSettings {
        settings: core.snapshot().settings,
    })
    .expect("second settings write should create a backup");
    drop(core);

    let settings_path = storage.path().join("settings.json");
    let mut newer: serde_json::Value = serde_json::from_slice(
        &fs::read(&settings_path).expect("live settings should be readable"),
    )
    .expect("live settings fixture should parse");
    newer["schemaVersion"] = serde_json::json!(2);
    let newer_bytes = serde_json::to_vec_pretty(&newer).expect("fixture should serialize");
    fs::write(&settings_path, &newer_bytes).expect("fixture should write newer schema");

    assert!(matches!(
        FormationLapCore::open_with_runtime(
            storage.path(),
            RunningProcessRuntime {
                next_identity: None,
                stop_requests: Arc::new(AtomicU64::new(0)),
            },
        ),
        Err(CoreError::UnsupportedSettingsSchema(2))
    ));
    assert_eq!(
        fs::read(&settings_path).expect("newer live bytes should remain untouched"),
        newer_bytes
    );
}

#[test]
fn corrupt_live_session_journal_recovers_only_to_a_recovery_offer() {
    let storage = TempStorage::new();
    let original = active_core(&storage, Arc::new(AtomicU64::new(0)));
    drop(original);
    let journal_path = storage.path().join("active-session.json");
    let backup_path = storage.path().join("backups").join("active-session.json");
    fs::copy(&journal_path, &backup_path).expect("fixture should retain a valid journal backup");
    fs::write(&journal_path, b"{ interrupted journal")
        .expect("fixture should corrupt live journal");

    let trace = Arc::new(RuntimeTrace::default());
    let mut recovered = FormationLapCore::open_with_runtime(
        storage.path(),
        RecoveryProcessRuntime {
            observation: ProcessObservation::Running {
                responsiveness: formation_lap_lib::ProcessResponsiveness::Responsive,
            },
            trace: Arc::clone(&trace),
        },
    )
    .expect("a valid journal backup should produce a Recovery Offer");
    assert_eq!(
        recovered.snapshot().session.state,
        SessionState::RecoveryAvailable
    );
    assert_eq!(trace.launches.load(Ordering::Relaxed), 0);
    assert_eq!(trace.stops.load(Ordering::Relaxed), 0);
    assert!(
        serde_json::from_slice::<serde_json::Value>(
            &fs::read(&journal_path).expect("recovered journal should be readable"),
        )
        .is_ok()
    );
    recovered
        .execute(AppCommand::DismissRecovery)
        .expect("dismissal should remain action-free");
    assert_eq!(trace.launches.load(Ordering::Relaxed), 0);
    assert_eq!(trace.stops.load(Ordering::Relaxed), 0);
}

#[test]
fn newer_live_session_journal_never_falls_back_to_an_older_backup() {
    let storage = TempStorage::new();
    let original = active_core(&storage, Arc::new(AtomicU64::new(0)));
    drop(original);
    let journal_path = storage.path().join("active-session.json");
    let backup_path = storage.path().join("backups").join("active-session.json");
    fs::copy(&journal_path, &backup_path).expect("fixture should retain a valid journal backup");
    let mut newer: serde_json::Value =
        serde_json::from_slice(&fs::read(&journal_path).expect("live journal should be readable"))
            .expect("live journal fixture should parse");
    newer["schemaVersion"] = serde_json::json!(2);
    let newer_bytes = serde_json::to_vec_pretty(&newer).expect("fixture should serialize");
    fs::write(&journal_path, &newer_bytes).expect("fixture should write newer journal");

    assert!(matches!(
        FormationLapCore::open_with_runtime(
            storage.path(),
            RunningProcessRuntime {
                next_identity: None,
                stop_requests: Arc::new(AtomicU64::new(0)),
            },
        ),
        Err(CoreError::UnsupportedSessionJournalSchema(2))
    ));
    assert_eq!(
        fs::read(&journal_path).expect("newer journal bytes should remain untouched"),
        newer_bytes
    );
}

#[test]
fn diagnostics_are_local_sanitized_and_bounded() {
    let storage = TempStorage::new();
    let runtime = RunningProcessRuntime {
        next_identity: None,
        stop_requests: Arc::new(AtomicU64::new(0)),
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("desktop test core should open");

    for _ in 0..2_000 {
        core.execute(AppCommand::RequestWindowClose)
            .expect("window-close decisions should be logged");
    }
    for _ in 0..200 {
        core.execute(AppCommand::RefreshProcesses)
            .expect("routine native monitoring should refresh");
    }

    let CommandOutcome::DiagnosticsExported { diagnostics } = core
        .execute(AppCommand::ExportDiagnostics)
        .expect("diagnostics should export")
    else {
        panic!("diagnostics command should return the local export");
    };
    assert_eq!(diagnostics.schema_version, 1);
    assert!(!diagnostics.telemetry_upload);
    assert_eq!(diagnostics.profile_count, 0);
    assert!(
        diagnostics.recent_events.len() <= 128,
        "the export must retain only a bounded recent event tail"
    );
    assert!(
        diagnostics
            .recent_events
            .iter()
            .all(|event| event.event != "process.refresh"),
        "successful native monitoring polls must not crowd useful actions out of the export"
    );

    let serialized =
        serde_json::to_string_pretty(&diagnostics).expect("diagnostics should serialize");
    assert!(!serialized.contains(storage.path().to_string_lossy().as_ref()));
    assert!(!serialized.contains("canonicalExecutablePath"));

    let log_files = fs::read_dir(storage.path().join("logs"))
        .expect("the local logs directory should exist")
        .map(|entry| entry.expect("log entry should be readable"))
        .collect::<Vec<_>>();
    assert!(
        log_files.len() <= 2,
        "only one live and one rotated log remain"
    );
    assert!(
        log_files.iter().all(|entry| {
            entry
                .metadata()
                .expect("log metadata should be readable")
                .len()
                <= 65_536
        }),
        "each local log file must stay within the configured bound"
    );
}

#[test]
fn recovery_rejects_pid_reuse_and_dismissal_performs_no_process_action() {
    let reused_storage = TempStorage::new();
    let original_stop_requests = Arc::new(AtomicU64::new(0));
    let original = active_core(&reused_storage, original_stop_requests);
    let original_identity = original.snapshot().application_processes[0]
        .identity
        .clone()
        .expect("active fixture should have a stable identity");
    drop(original);

    let replaced_trace = Arc::new(RuntimeTrace::default());
    let replaced_identity = ProcessIdentity {
        pid: original_identity.pid,
        creation_time: "different-creation-time".to_owned(),
        canonical_executable_path: original_identity.canonical_executable_path.clone(),
    };
    let replaced = FormationLapCore::open_with_runtime(
        reused_storage.path(),
        RecoveryProcessRuntime {
            observation: ProcessObservation::Replaced {
                current_identity: replaced_identity,
            },
            trace: Arc::clone(&replaced_trace),
        },
    )
    .expect("a reused PID should be treated as a stale journal");
    assert_eq!(replaced.snapshot().session.state, SessionState::Idle);
    assert!(
        !reused_storage.path().join("active-session.json").exists(),
        "the stale journal should be removed"
    );
    assert_eq!(replaced_trace.observations.load(Ordering::Relaxed), 1);
    assert_eq!(replaced_trace.launches.load(Ordering::Relaxed), 0);
    assert_eq!(replaced_trace.stops.load(Ordering::Relaxed), 0);

    let dismiss_storage = TempStorage::new();
    let running = active_core(&dismiss_storage, Arc::new(AtomicU64::new(0)));
    drop(running);
    let dismiss_trace = Arc::new(RuntimeTrace::default());
    let mut recovery = FormationLapCore::open_with_runtime(
        dismiss_storage.path(),
        RecoveryProcessRuntime {
            observation: ProcessObservation::Running {
                responsiveness: formation_lap_lib::ProcessResponsiveness::Responsive,
            },
            trace: Arc::clone(&dismiss_trace),
        },
    )
    .expect("the exact live identity should produce a Recovery Offer");
    assert_eq!(
        recovery.snapshot().session.state,
        SessionState::RecoveryAvailable
    );
    recovery
        .execute(AppCommand::DismissRecovery)
        .expect("Recovery Offer should dismiss");
    assert_eq!(recovery.snapshot().session.state, SessionState::Idle);
    assert_eq!(dismiss_trace.observations.load(Ordering::Relaxed), 1);
    assert_eq!(dismiss_trace.launches.load(Ordering::Relaxed), 0);
    assert_eq!(dismiss_trace.stops.load(Ordering::Relaxed), 0);
}
