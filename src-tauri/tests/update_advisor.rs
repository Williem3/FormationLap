use formation_lap_lib::{
    AppCommand, ApplicationRequirement, CatalogUpdateProvider, CommandOutcome, ConsoleVisibility,
    FormationLapCore, FormationLapInstallDecision, GracefulStopResult, LaunchRecipe, LaunchSource,
    ProcessIdentity, ProcessObservation, ProcessOutput, ProcessRuntime, ProcessRuntimeError,
    RacingProfile, SessionState, ShutdownStrategy, SupportingApplication, UpdateChannel,
    UpdateCheckDecision, UpdateCheckResult, UpdateCheckTrigger, UpdateStatus,
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
            "formation-lap-update-test-{}-{timestamp}-{unique}",
            process::id()
        ));
        fs::create_dir_all(&path).expect("temporary update storage should be created");
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
            .ok_or_else(|| ProcessRuntimeError::new("the update fixture is exhausted"))
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

fn core_with_launchable_profile(
    storage: &TempStorage,
    stop_requests: Arc<AtomicU64>,
) -> (FormationLapCore, String) {
    let executable_path = std::env::current_exe()
        .expect("test executable should be available")
        .canonicalize()
        .expect("test executable should canonicalize");
    let identity = ProcessIdentity {
        pid: 920,
        creation_time: "update-fixture".to_owned(),
        canonical_executable_path: executable_path.to_string_lossy().into_owned(),
    };
    let mut core = FormationLapCore::open_with_runtime(
        storage.path(),
        RunningProcessRuntime {
            next_identity: Some(identity),
            stop_requests,
        },
    )
    .expect("update fixture core should open");
    let CommandOutcome::ProfileCreated { profile_id } = core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Update fixture".to_owned(),
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
    (core, profile_id)
}

#[test]
fn automatic_update_checks_are_opt_in_and_an_explicit_true_persists() {
    let storage = TempStorage::new();
    let mut core = FormationLapCore::open(storage.path()).expect("update test core should open");

    assert!(!core.snapshot().settings.automatic_update_checks);
    assert_eq!(
        core.snapshot().settings.update_channel,
        UpdateChannel::Stable
    );
    assert_eq!(
        core.execute(AppCommand::PrepareUpdateCheck {
            trigger: UpdateCheckTrigger::Automatic,
            now_unix_seconds: 999_999,
        })
        .expect("the default automatic check should be decided"),
        CommandOutcome::UpdateCheckPrepared {
            decision: UpdateCheckDecision::Disabled
        }
    );
    let mut settings = core.snapshot().settings;
    settings.automatic_update_checks = true;
    core.execute(AppCommand::UpdateSettings { settings })
        .expect("explicit update consent should save");

    let decision = core
        .execute(AppCommand::PrepareUpdateCheck {
            trigger: UpdateCheckTrigger::Automatic,
            now_unix_seconds: 1_000_000,
        })
        .expect("the first automatic update check should be prepared");
    let CommandOutcome::UpdateCheckPrepared {
        decision: UpdateCheckDecision::Planned(plan),
    } = decision
    else {
        panic!("the first automatic update check should have a plan");
    };
    assert_eq!(plan.channel, UpdateChannel::Stable);

    core.execute(AppCommand::CompleteUpdateCheck {
        result: UpdateCheckResult {
            request_id: plan.request_id,
            formation_lap: UpdateStatus::Current {
                current_version: "0.1.0".to_owned(),
            },
            applications: Vec::new(),
        },
    })
    .expect("the completed check should update visible advice");
    assert_eq!(
        core.snapshot().updates.formation_lap,
        UpdateStatus::Current {
            current_version: "0.1.0".to_owned()
        }
    );
    drop(core);

    let mut reopened =
        FormationLapCore::open(storage.path()).expect("update schedule should reopen");
    assert!(
        reopened.snapshot().settings.automatic_update_checks,
        "an explicitly saved true must survive upgrade and restart"
    );
    assert_eq!(
        reopened
            .execute(AppCommand::PrepareUpdateCheck {
                trigger: UpdateCheckTrigger::Automatic,
                now_unix_seconds: 1_000_000 + 86_399,
            })
            .expect("a same-day automatic check should be decided"),
        CommandOutcome::UpdateCheckPrepared {
            decision: UpdateCheckDecision::NotDue
        }
    );

    let CommandOutcome::UpdateCheckPrepared {
        decision: UpdateCheckDecision::Planned(next_plan),
    } = reopened
        .execute(AppCommand::PrepareUpdateCheck {
            trigger: UpdateCheckTrigger::Automatic,
            now_unix_seconds: 1_000_000 + 86_400,
        })
        .expect("the next-day automatic check should be prepared")
    else {
        panic!("the daily interval should produce a new plan");
    };
    assert_eq!(next_plan.channel, UpdateChannel::Stable);
}

#[test]
fn disabled_automatic_checks_still_allow_an_explicit_beta_check() {
    let storage = TempStorage::new();
    let mut core = FormationLapCore::open(storage.path()).expect("update test core should open");
    let mut settings = core.snapshot().settings;
    settings.automatic_update_checks = false;
    settings.update_channel = UpdateChannel::Beta;
    core.execute(AppCommand::UpdateSettings { settings })
        .expect("update preferences should save");

    assert_eq!(
        core.execute(AppCommand::PrepareUpdateCheck {
            trigger: UpdateCheckTrigger::Automatic,
            now_unix_seconds: 2_000_000,
        })
        .expect("disabled automatic check should be decided"),
        CommandOutcome::UpdateCheckPrepared {
            decision: UpdateCheckDecision::Disabled
        }
    );

    let CommandOutcome::UpdateCheckPrepared {
        decision: UpdateCheckDecision::Planned(plan),
    } = core
        .execute(AppCommand::PrepareUpdateCheck {
            trigger: UpdateCheckTrigger::Manual,
            now_unix_seconds: 2_000_000,
        })
        .expect("manual check should remain available")
    else {
        panic!("manual check should produce a plan");
    };
    assert_eq!(plan.channel, UpdateChannel::Beta);
}

#[test]
fn race_safe_session_blocks_network_and_defers_in_flight_results_until_idle() {
    let storage = TempStorage::new();
    let stop_requests = Arc::new(AtomicU64::new(0));
    let (mut core, profile_id) = core_with_launchable_profile(&storage, Arc::clone(&stop_requests));

    let CommandOutcome::UpdateCheckPrepared {
        decision: UpdateCheckDecision::Planned(plan),
    } = core
        .execute(AppCommand::PrepareUpdateCheck {
            trigger: UpdateCheckTrigger::Manual,
            now_unix_seconds: 3_000_000,
        })
        .expect("idle manual check should be prepared")
    else {
        panic!("idle manual check should produce a plan");
    };

    core.execute(AppCommand::StartSession { profile_id })
        .expect("Session should start");
    core.execute(AppCommand::RefreshProcesses)
        .expect("Session should become Active");
    assert_eq!(core.snapshot().session.state, SessionState::Active);
    assert_eq!(
        core.execute(AppCommand::PrepareUpdateCheck {
            trigger: UpdateCheckTrigger::Manual,
            now_unix_seconds: 3_000_001,
        })
        .expect("Active-Session check should be deferred"),
        CommandOutcome::UpdateCheckPrepared {
            decision: UpdateCheckDecision::Deferred
        }
    );

    core.execute(AppCommand::CompleteUpdateCheck {
        result: UpdateCheckResult {
            request_id: plan.request_id,
            formation_lap: UpdateStatus::UpdateAvailable {
                current_version: "0.1.0".to_owned(),
                latest_version: "1.0.0".to_owned(),
            },
            applications: Vec::new(),
        },
    })
    .expect("in-flight result should be accepted quietly");
    assert_eq!(
        core.snapshot().updates.formation_lap,
        UpdateStatus::default()
    );
    assert!(core.snapshot().updates.result_deferred);

    core.execute(AppCommand::CloseSession)
        .expect("Session close should begin");
    core.execute(AppCommand::RefreshProcesses)
        .expect("Session close should finish");
    assert_eq!(core.snapshot().session.state, SessionState::Idle);
    assert_eq!(
        core.snapshot().updates.formation_lap,
        UpdateStatus::UpdateAvailable {
            current_version: "0.1.0".to_owned(),
            latest_version: "1.0.0".to_owned(),
        }
    );
    assert!(!core.snapshot().updates.result_deferred);
    assert_eq!(stop_requests.load(Ordering::Relaxed), 1);
}

#[test]
fn signed_formation_lap_installation_can_prepare_only_while_idle() {
    let storage = TempStorage::new();
    let (mut core, profile_id) =
        core_with_launchable_profile(&storage, Arc::new(AtomicU64::new(0)));
    let CommandOutcome::UpdateCheckPrepared {
        decision: UpdateCheckDecision::Planned(plan),
    } = core
        .execute(AppCommand::PrepareUpdateCheck {
            trigger: UpdateCheckTrigger::Manual,
            now_unix_seconds: 4_000_000,
        })
        .expect("idle update check should be prepared")
    else {
        panic!("idle update check should produce a plan");
    };
    core.execute(AppCommand::CompleteUpdateCheck {
        result: UpdateCheckResult {
            request_id: plan.request_id,
            formation_lap: UpdateStatus::UpdateAvailable {
                current_version: "0.1.0".to_owned(),
                latest_version: "1.0.0".to_owned(),
            },
            applications: Vec::new(),
        },
    })
    .expect("available signed update should be recorded");

    assert_eq!(
        core.execute(AppCommand::PrepareFormationLapInstall)
            .expect("idle install should be decided"),
        CommandOutcome::FormationLapInstallPrepared {
            decision: FormationLapInstallDecision::Ready {
                latest_version: "1.0.0".to_owned(),
            }
        }
    );

    assert!(
        core.execute(AppCommand::StartSession {
            profile_id: profile_id.clone(),
        })
        .is_err(),
        "the core-owned install lease must exclude Session start"
    );
    core.execute(AppCommand::CancelFormationLapInstall {
        expected_version: "1.0.0".to_owned(),
    })
    .expect("a failed or cancelled installer should release its lease");
    core.execute(AppCommand::StartSession { profile_id })
        .expect("Session should start after the install lease is released");
    core.execute(AppCommand::RefreshProcesses)
        .expect("Session should become Active");
    assert_eq!(core.snapshot().session.state, SessionState::Active);
    assert_eq!(
        core.execute(AppCommand::PrepareFormationLapInstall)
            .expect("Active-Session install should be decided"),
        CommandOutcome::FormationLapInstallPrepared {
            decision: FormationLapInstallDecision::Deferred
        }
    );
}

#[test]
fn update_plan_contains_only_configured_applications_and_their_curated_provider_identity() {
    let storage = TempStorage::new();
    let (mut core, _) = core_with_launchable_profile(&storage, Arc::new(AtomicU64::new(0)));
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("fixture profile should be selected");
    profile.supporting_applications = [
        ("lmuffb", "LMUFFB"),
        ("trading", "Trading Paints"),
        ("simhub", "SimHub"),
        ("custom", "Custom tool"),
    ]
    .into_iter()
    .map(|(id, name)| {
        let mut application = profile.primary_sim.clone();
        application.id = id.to_owned();
        application.name = name.to_owned();
        SupportingApplication {
            application,
            requirement: ApplicationRequirement::Optional,
            keep_running: false,
        }
    })
    .collect();
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("configured applications should save");

    let CommandOutcome::UpdateCheckPrepared {
        decision: UpdateCheckDecision::Planned(plan),
    } = core
        .execute(AppCommand::PrepareUpdateCheck {
            trigger: UpdateCheckTrigger::Manual,
            now_unix_seconds: 5_000_000,
        })
        .expect("manual check should be prepared")
    else {
        panic!("manual check should produce a provider plan");
    };

    assert_eq!(
        plan.applications
            .iter()
            .map(|target| (target.name.as_str(), target.provider.clone()))
            .collect::<Vec<_>>(),
        vec![
            (
                "LMUFFB",
                Some(CatalogUpdateProvider::GitHubReleases {
                    repository: "coasting-nc/LMUFFB".to_owned(),
                }),
            ),
            (
                "Trading Paints",
                Some(CatalogUpdateProvider::Winget {
                    package_id: "Rhinode.TradingPaints".to_owned(),
                }),
            ),
            (
                "SimHub",
                Some(CatalogUpdateProvider::OfficialPage {
                    url: "https://www.simhubdash.com/download-2/".to_owned(),
                }),
            ),
            ("Custom tool", None),
        ]
    );
}
