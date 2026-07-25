use formation_lap_lib::{
    AppCommand, ApplicationRequirement, CommandOutcome, ConsoleVisibility,
    DevelopmentPrivilegeBroker, ELEVATED_HELPER_PROTOCOL_VERSION, ElevatedHelperRequest,
    ElevatedHelperResponse, ElevatedOperation, ElevatedOperationResult, ElevatedRequestValidator,
    FormationLapCore, GracefulStopResult, HelperProtocolError, HelperValidationContext,
    LaunchRecipe, LaunchSource, MAX_ELEVATED_OPERATIONS, PrivilegeBroker, ProcessIdentity,
    ProcessObservation, ProcessOutput, ProcessResponsiveness, ProcessRuntime, ProcessRuntimeError,
    RacingProfile, SessionState, ShutdownStrategy, SupportingApplication, WindowsPrivilegeBroker,
    decode_helper_request,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn canonical_current_executable() -> String {
    std::env::current_exe()
        .expect("the test executable should have a path")
        .canonicalize()
        .expect("the test executable path should canonicalize")
        .to_string_lossy()
        .into_owned()
}

fn valid_request() -> (ElevatedHelperRequest, HelperValidationContext) {
    let parent_identity = ProcessIdentity {
        pid: 4_242,
        creation_time: "100".to_owned(),
        canonical_executable_path: canonical_current_executable(),
    };
    (
        ElevatedHelperRequest {
            protocol_version: ELEVATED_HELPER_PROTOCOL_VERSION,
            parent_identity: parent_identity.clone(),
            nonce: "2d24b92a-3799-4acd-a8eb-183189ad5838".to_owned(),
            current_user_id: "S-1-5-21-formation-lap-test".to_owned(),
            operations: vec![ElevatedOperation::Launch {
                executable_path: canonical_current_executable(),
                arguments: vec!["--safe-fixture".to_owned()],
                working_directory: Some(
                    Path::new(&canonical_current_executable())
                        .parent()
                        .expect("test executable should have a parent")
                        .to_string_lossy()
                        .into_owned(),
                ),
                monitored_process: None,
                console_visibility: ConsoleVisibility::Hidden,
                startup_timeout_seconds: 30,
            }],
        },
        HelperValidationContext {
            current_user_id: "S-1-5-21-formation-lap-test".to_owned(),
            parent_identity,
            helper_process_id: 7_777,
            operation_process_identities: Vec::new(),
            same_interactive_session: true,
            expected_application_path: true,
            release_identity_verified: true,
        },
    )
}

#[test]
fn helper_rejects_a_protocol_version_mismatch_before_any_operation() {
    let (mut request, context) = valid_request();
    request.protocol_version += 1;

    let error = ElevatedRequestValidator::default()
        .validate(&request, &context)
        .expect_err("an unsupported helper protocol must be rejected");

    assert_eq!(
        error,
        HelperProtocolError::VersionMismatch {
            expected: ELEVATED_HELPER_PROTOCOL_VERSION,
            received: request.protocol_version,
        }
    );
}

#[test]
fn helper_accepts_one_complete_canonical_typed_batch_once() {
    let (request, context) = valid_request();
    let mut validator = ElevatedRequestValidator::default();

    validator
        .validate(&request, &context)
        .expect("a canonical typed batch should validate");
    assert_eq!(
        validator
            .validate(&request, &context)
            .expect_err("a consumed nonce must not validate twice"),
        HelperProtocolError::ReplayedNonce
    );
}

#[test]
fn helper_rejects_the_wrong_user_and_wrong_parent_identity() {
    let (request, mut context) = valid_request();
    context.current_user_id = "S-1-5-21-another-user".to_owned();
    assert_eq!(
        ElevatedRequestValidator::default()
            .validate(&request, &context)
            .expect_err("a different current user must be rejected"),
        HelperProtocolError::WrongUser
    );

    let (request, mut context) = valid_request();
    context.parent_identity.creation_time = "101".to_owned();
    assert_eq!(
        ElevatedRequestValidator::default()
            .validate(&request, &context)
            .expect_err("a PID without the same creation time is not the same parent"),
        HelperProtocolError::WrongParentIdentity
    );
}

#[test]
fn helper_rejects_a_caller_outside_the_authenticated_release_boundary() {
    let (request, mut context) = valid_request();
    context.same_interactive_session = false;
    assert_eq!(
        ElevatedRequestValidator::default()
            .validate(&request, &context)
            .expect_err("a caller in another interactive Session must be rejected"),
        HelperProtocolError::WrongInteractiveSession
    );

    let (request, mut context) = valid_request();
    context.expected_application_path = false;
    assert_eq!(
        ElevatedRequestValidator::default()
            .validate(&request, &context)
            .expect_err("a renamed or non-sibling caller must be rejected"),
        HelperProtocolError::UnexpectedApplicationPath
    );

    let (request, mut context) = valid_request();
    context.release_identity_verified = false;
    assert_eq!(
        ElevatedRequestValidator::default()
            .validate(&request, &context)
            .expect_err("an unverified release identity must be rejected"),
        HelperProtocolError::UnverifiedReleaseIdentity
    );
}

#[test]
fn helper_rejects_noncanonical_and_shell_targets() {
    let (mut request, context) = valid_request();
    let canonical = canonical_current_executable();
    let executable_path = Path::new(&canonical);
    let parent = executable_path
        .parent()
        .expect("test executable should have a parent");
    let noncanonical = format!(
        "{}\\..\\{}\\{}",
        parent.to_string_lossy(),
        parent
            .file_name()
            .expect("test executable parent should have a name")
            .to_string_lossy(),
        executable_path
            .file_name()
            .expect("test executable should have a file name")
            .to_string_lossy()
    );
    request.operations[0] = ElevatedOperation::Launch {
        executable_path: noncanonical.clone(),
        arguments: Vec::new(),
        working_directory: None,
        monitored_process: None,
        console_visibility: ConsoleVisibility::Hidden,
        startup_timeout_seconds: 30,
    };
    assert_eq!(
        ElevatedRequestValidator::default()
            .validate(&request, &context)
            .expect_err("a path with unresolved components must be rejected"),
        HelperProtocolError::NonCanonicalPath(noncanonical)
    );

    let (mut request, context) = valid_request();
    request.operations[0] = ElevatedOperation::Launch {
        executable_path: canonical_system_executable("cmd.exe"),
        arguments: vec!["/c".to_owned(), "echo unsafe".to_owned()],
        working_directory: None,
        monitored_process: None,
        console_visibility: ConsoleVisibility::Hidden,
        startup_timeout_seconds: 30,
    };
    assert!(matches!(
        ElevatedRequestValidator::default()
            .validate(&request, &context)
            .expect_err("the helper must not become an arbitrary shell"),
        HelperProtocolError::InvalidExecutable(message)
            if message.contains("shell and script hosts")
    ));
}

#[test]
fn helper_rejects_raw_shell_documents_and_out_of_scope_operations() {
    for document in [
        br#"{"kind":"shell","command":"whoami"}"#.as_slice(),
        br#"{
          "protocolVersion":1,
          "parentIdentity":{"pid":1,"creationTime":"1","canonicalExecutablePath":"C:\\app.exe"},
          "nonce":"2d24b92a-3799-4acd-a8eb-183189ad5838",
          "currentUserId":"S-1-5-21-test",
          "operations":[{"kind":"writeFile","path":"C:\\target","contents":"unsafe"}]
        }"#,
    ] {
        assert!(matches!(
            decode_helper_request(document)
                .expect_err("untyped and out-of-scope documents must be rejected"),
            HelperProtocolError::InvalidDocument(_)
        ));
    }
}

#[test]
fn helper_rejects_oversized_batches_and_line_bearing_arguments() {
    let (mut request, context) = valid_request();
    request.operations = vec![request.operations[0].clone(); MAX_ELEVATED_OPERATIONS + 1];
    assert_eq!(
        ElevatedRequestValidator::default()
            .validate(&request, &context)
            .expect_err("an oversized operation batch must be rejected"),
        HelperProtocolError::BatchTooLarge {
            maximum: MAX_ELEVATED_OPERATIONS,
            received: MAX_ELEVATED_OPERATIONS + 1,
        }
    );

    let (mut request, context) = valid_request();
    request.operations[0] = ElevatedOperation::Launch {
        executable_path: canonical_current_executable(),
        arguments: vec!["safe\r\nwhoami".to_owned()],
        working_directory: None,
        monitored_process: None,
        console_visibility: ConsoleVisibility::Hidden,
        startup_timeout_seconds: 30,
    };
    assert!(matches!(
        ElevatedRequestValidator::default()
            .validate(&request, &context)
            .expect_err("arguments cannot smuggle another line of shell text"),
        HelperProtocolError::InvalidArguments(_)
    ));
}

#[test]
fn helper_rejects_a_reused_pid_and_protects_its_parent() {
    let (mut request, context) = valid_request();
    let stale_identity = ProcessIdentity {
        pid: 8_888,
        creation_time: "old".to_owned(),
        canonical_executable_path: canonical_current_executable(),
    };
    request.operations[0] = ElevatedOperation::ForceTerminate {
        process_identity: stale_identity.clone(),
    };
    assert_eq!(
        ElevatedRequestValidator::default()
            .validate(&request, &context)
            .expect_err("a PID without the observed stable identity must be rejected"),
        HelperProtocolError::WrongProcessIdentity(stale_identity.pid)
    );

    let (mut request, context) = valid_request();
    request.operations[0] = ElevatedOperation::ForceTerminate {
        process_identity: request.parent_identity.clone(),
    };
    assert_eq!(
        ElevatedRequestValidator::default()
            .validate(&request, &context)
            .expect_err("the authenticated parent must not be a termination target"),
        HelperProtocolError::ProtectedProcess(request.parent_identity.pid)
    );
}

fn canonical_system_executable(name: &str) -> String {
    PathBuf::from(std::env::var_os("SystemRoot").expect("Windows should define SystemRoot"))
        .join("System32")
        .join(name)
        .canonicalize()
        .expect("the system executable should canonicalize")
        .to_string_lossy()
        .into_owned()
}

#[derive(Default)]
struct PrivilegedStartupRuntime;

impl ProcessRuntime for PrivilegedStartupRuntime {
    fn matching_processes(
        &mut self,
        _recipe: &LaunchRecipe,
    ) -> Result<Vec<ProcessIdentity>, ProcessRuntimeError> {
        Ok(Vec::new())
    }

    fn launch(&mut self, _recipe: &LaunchRecipe) -> Result<ProcessIdentity, ProcessRuntimeError> {
        panic!("an elevated launch must go through PrivilegeBroker")
    }

    fn observe(
        &mut self,
        _identity: &ProcessIdentity,
    ) -> Result<ProcessObservation, ProcessRuntimeError> {
        Ok(ProcessObservation::Running {
            responsiveness: ProcessResponsiveness::Responsive,
        })
    }

    fn request_graceful_stop(
        &mut self,
        _identity: &ProcessIdentity,
        _strategy: &ShutdownStrategy,
    ) -> Result<GracefulStopResult, ProcessRuntimeError> {
        panic!("an elevated stop must go through PrivilegeBroker")
    }

    fn wait_for_exit(
        &mut self,
        _identity: &ProcessIdentity,
        _timeout: Duration,
    ) -> Result<bool, ProcessRuntimeError> {
        panic!("an elevated helper owns its bounded graceful-stop wait")
    }

    fn force_stop(&mut self, _identity: &ProcessIdentity) -> Result<(), ProcessRuntimeError> {
        panic!("an elevated force stop must go through PrivilegeBroker")
    }

    fn read_output(
        &mut self,
        _identity: &ProcessIdentity,
    ) -> Result<ProcessOutput, ProcessRuntimeError> {
        Ok(ProcessOutput::default())
    }
}

struct TempStorage(PathBuf);

impl TempStorage {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow the Unix epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("formation-lap-m7-{}-{unique}", std::process::id()));
        fs::create_dir_all(&path).expect("temporary storage should be created");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempStorage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn one_session_startup_batches_all_elevated_launches_through_one_broker_request() {
    let storage = TempStorage::new();
    let broker = DevelopmentPrivilegeBroker::default();
    let observed_broker = broker.clone();
    let executable_path = canonical_current_executable();
    let identities = [11_001, 11_002].map(|pid| ProcessIdentity {
        pid,
        creation_time: format!("created-{pid}"),
        canonical_executable_path: executable_path.clone(),
    });
    broker.queue_response(Ok(ElevatedHelperResponse {
        protocol_version: ELEVATED_HELPER_PROTOCOL_VERSION,
        nonce: "development-adapter".to_owned(),
        accepted: true,
        error: None,
        results: identities
            .iter()
            .cloned()
            .map(|process_identity| ElevatedOperationResult::Launched { process_identity })
            .collect(),
    }));
    let mut core = FormationLapCore::open_with_runtime_and_privilege_broker(
        storage.path(),
        PrivilegedStartupRuntime,
        broker,
    )
    .expect("the core should open with the approved test adapters");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Elevated Rig".to_owned(),
            primary_sim_name: "Primary".to_owned(),
        })
        .expect("the profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("unexpected create outcome: {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("the created profile should be selected");
    profile.primary_sim.launch_recipe = direct_recipe(&executable_path, false);
    profile.supporting_applications = ["Telemetry", "Switcher"]
        .into_iter()
        .map(|name| SupportingApplication {
            application: formation_lap_lib::ProfileApplication {
                id: name.to_ascii_lowercase(),
                name: name.to_owned(),
                launch_recipe: direct_recipe(&executable_path, true),
                path_needs_repair: false,
            },
            requirement: ApplicationRequirement::Required,
            keep_running: false,
        })
        .collect();
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(RacingProfile {
            id: profile_id.clone(),
            ..profile
        }),
    })
    .expect("the elevated Startup Sequence should save");

    assert_eq!(
        core.execute(AppCommand::StartSession {
            profile_id: profile_id.clone(),
        })
        .expect("the Session should request its elevated batch"),
        CommandOutcome::SessionStartRequested { profile_id }
    );

    let batches = observed_broker.recorded_batches();
    assert_eq!(batches.len(), 1, "one Session should request UAC once");
    assert_eq!(batches[0].len(), 2);
    assert!(
        batches[0]
            .iter()
            .all(|operation| matches!(operation, ElevatedOperation::Launch { .. }))
    );
    let snapshot = core.snapshot();
    assert_eq!(snapshot.application_processes.len(), 1);
    assert_eq!(
        snapshot.application_processes[0].identity,
        Some(identities[0].clone())
    );
}

#[test]
fn cancelling_startup_closes_every_process_from_the_elevated_launch_batch() {
    let storage = TempStorage::new();
    let broker = DevelopmentPrivilegeBroker::default();
    let observed_broker = broker.clone();
    let executable_path = canonical_current_executable();
    let identities = [11_101, 11_102].map(|pid| ProcessIdentity {
        pid,
        creation_time: format!("created-{pid}"),
        canonical_executable_path: executable_path.clone(),
    });
    broker.queue_response(Ok(ElevatedHelperResponse {
        protocol_version: ELEVATED_HELPER_PROTOCOL_VERSION,
        nonce: "development-start".to_owned(),
        accepted: true,
        error: None,
        results: identities
            .iter()
            .cloned()
            .map(|process_identity| ElevatedOperationResult::Launched { process_identity })
            .collect(),
    }));
    broker.queue_response(Ok(ElevatedHelperResponse {
        protocol_version: ELEVATED_HELPER_PROTOCOL_VERSION,
        nonce: "development-cancel".to_owned(),
        accepted: true,
        error: None,
        results: vec![
            ElevatedOperationResult::GracefulStopRequested {
                requested: true,
                exited: true,
            };
            2
        ],
    }));
    let mut core = FormationLapCore::open_with_runtime_and_privilege_broker(
        storage.path(),
        PrivilegedStartupRuntime,
        broker,
    )
    .expect("the core should open with the approved test adapters");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Elevated Cancel".to_owned(),
            primary_sim_name: "Primary".to_owned(),
        })
        .expect("the profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("unexpected create outcome: {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("the created profile should be selected");
    profile.primary_sim.launch_recipe = direct_recipe(&executable_path, false);
    profile.supporting_applications = ["Telemetry", "Switcher"]
        .into_iter()
        .map(|name| SupportingApplication {
            application: formation_lap_lib::ProfileApplication {
                id: name.to_ascii_lowercase(),
                name: name.to_owned(),
                launch_recipe: direct_recipe(&executable_path, true),
                path_needs_repair: false,
            },
            requirement: ApplicationRequirement::Required,
            keep_running: false,
        })
        .collect();
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(RacingProfile {
            id: profile_id.clone(),
            ..profile
        }),
    })
    .expect("the elevated Startup Sequence should save");

    core.execute(AppCommand::StartSession { profile_id })
        .expect("the Session should request its elevated batch");
    core.execute(AppCommand::CancelStartup)
        .expect("the user should be able to cancel startup");
    core.execute(AppCommand::RefreshProcesses)
        .expect("cancellation should close current and prepared elevated Processes");

    assert_eq!(core.snapshot().session.state, SessionState::Idle);
    assert!(
        core.snapshot()
            .application_processes
            .iter()
            .all(|process| { process.ownership.is_none() && process.identity.is_none() })
    );
    let batches = observed_broker.recorded_batches();
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[1].len(), 2);
    assert!(
        batches[1]
            .iter()
            .all(|operation| matches!(operation, ElevatedOperation::GracefulStop { .. }))
    );
}

fn direct_recipe(executable_path: &str, elevated: bool) -> LaunchRecipe {
    LaunchRecipe {
        source: LaunchSource::DirectExecutable {
            executable_path: executable_path.to_owned(),
        },
        arguments: Vec::new(),
        working_directory: None,
        monitored_process: None,
        monitored_executable_path: None,
        console_visibility: ConsoleVisibility::Hidden,
        elevated,
        startup_timeout_seconds: 30,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::CloseWindows,
    }
}

#[test]
#[ignore = "manual Windows UAC evidence; run explicitly after building the helper and fixtures"]
fn manual_uac_helper_launches_and_closes_an_elevated_window_fixture() {
    let storage = TempStorage::new();
    let target_debug = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug");
    let helper_path = target_debug.join("formation-lap-elevated-helper.exe");
    let fixture_path = target_debug.join("formation-lap-process-fixture.exe");
    let report_path = storage.path().join("elevated-fixture-report.json");
    let receipt_path = target_debug.join("m7-uac-smoke-receipt.json");
    let write_receipt = |stage: &str, error: Option<&str>| {
        fs::write(
            &receipt_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "schemaVersion": 1,
                "stage": stage,
                "error": error,
            }))
            .expect("the UAC receipt should serialize"),
        )
        .expect("the ignored manual test should write its local receipt");
    };
    let mut broker = WindowsPrivilegeBroker::from_helper_path(&helper_path)
        .expect("the separately built helper should be available");

    write_receipt("launchPrompt", None);
    println!("manual M7 smoke: approve elevated fixture launch");
    let launch = match broker.execute(&[ElevatedOperation::Launch {
        executable_path: fixture_path
            .canonicalize()
            .expect("the fixture should be built")
            .to_string_lossy()
            .into_owned(),
        arguments: vec![
            "--report".to_owned(),
            report_path.to_string_lossy().into_owned(),
            "--lifetime-ms".to_owned(),
            "60000".to_owned(),
            "--window-state".to_owned(),
            "responsive".to_owned(),
        ],
        working_directory: Some(
            storage
                .path()
                .canonicalize()
                .expect("temporary storage should canonicalize")
                .to_string_lossy()
                .into_owned(),
        ),
        monitored_process: None,
        console_visibility: ConsoleVisibility::Hidden,
        startup_timeout_seconds: 10,
    }]) {
        Ok(response) => response,
        Err(error) => {
            write_receipt("launchFailed", Some(&error.to_string()));
            panic!("approving the first UAC prompt should launch the fixture: {error}");
        }
    };
    let identity = match launch.results.as_slice() {
        [ElevatedOperationResult::Launched { process_identity }] => process_identity.clone(),
        other => panic!("unexpected elevated launch results: {other:?}"),
    };

    write_receipt("closePrompt", None);
    println!("manual M7 smoke: approve elevated fixture close");
    let close = match broker.execute(&[ElevatedOperation::GracefulStop {
        process_identity: identity,
        strategy: ShutdownStrategy::CloseWindows,
    }]) {
        Ok(response) => response,
        Err(error) => {
            write_receipt("closeFailed", Some(&error.to_string()));
            panic!("approving the second UAC prompt should close the fixture: {error}");
        }
    };
    assert!(matches!(
        close.results.as_slice(),
        [ElevatedOperationResult::GracefulStopRequested {
            requested: true,
            exited: true,
        }]
    ));
    assert!(
        report_path.exists(),
        "the elevated fixture should have written its local report"
    );
    write_receipt("passed", None);
    println!("manual M7 smoke: elevated launch and close passed");
}

#[cfg(feature = "process-fixtures")]
#[test]
fn one_shot_helper_exits_after_an_accepted_or_rejected_request() {
    let storage = TempStorage::new();
    let target_debug = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug");
    let helper_path = target_debug.join("formation-lap-elevated-helper.exe");
    let fixture_path = target_debug
        .join("formation-lap-process-fixture.exe")
        .canonicalize()
        .expect("the fixture should be built");
    let mut broker = WindowsPrivilegeBroker::from_helper_path(&helper_path)
        .expect("the separately built helper should be available");
    let response = broker
        .execute_without_elevation_for_test(&[ElevatedOperation::Launch {
            executable_path: fixture_path.to_string_lossy().into_owned(),
            arguments: vec![
                "--report".to_owned(),
                storage
                    .path()
                    .join("one-shot-report.json")
                    .to_string_lossy()
                    .into_owned(),
                "--lifetime-ms".to_owned(),
                "60000".to_owned(),
            ],
            working_directory: Some(
                storage
                    .path()
                    .canonicalize()
                    .expect("temporary storage should canonicalize")
                    .to_string_lossy()
                    .into_owned(),
            ),
            monitored_process: None,
            console_visibility: ConsoleVisibility::Hidden,
            startup_timeout_seconds: 10,
        }])
        .expect("the helper should execute one accepted typed request and exit");
    let process_identity = match response.results.as_slice() {
        [ElevatedOperationResult::Launched { process_identity }] => process_identity.clone(),
        other => panic!("unexpected one-shot launch results: {other:?}"),
    };
    let terminated = broker
        .execute_without_elevation_for_test(&[ElevatedOperation::ForceTerminate {
            process_identity,
        }])
        .expect("the next one-shot helper should terminate the exact fixture and exit");
    assert!(matches!(
        terminated.results.as_slice(),
        [ElevatedOperationResult::ForceTerminated]
    ));

    let fixture_parent = fixture_path
        .parent()
        .expect("the fixture should have a parent");
    let noncanonical = format!(
        "{}\\..\\{}\\{}",
        fixture_parent.to_string_lossy(),
        fixture_parent
            .file_name()
            .expect("the fixture parent should have a name")
            .to_string_lossy(),
        fixture_path
            .file_name()
            .expect("the fixture should have a name")
            .to_string_lossy()
    );
    let error = broker
        .execute_without_elevation_for_test(&[ElevatedOperation::Launch {
            executable_path: noncanonical,
            arguments: Vec::new(),
            working_directory: None,
            monitored_process: None,
            console_visibility: ConsoleVisibility::Hidden,
            startup_timeout_seconds: 10,
        }])
        .expect_err("the helper should reject the whole invalid request and exit");
    assert!(error.to_string().contains("not canonical"));
}

#[test]
fn elevated_manual_restart_routes_close_and_relaunch_through_the_broker() {
    let storage = TempStorage::new();
    let broker = DevelopmentPrivilegeBroker::default();
    let observed_broker = broker.clone();
    let executable_path = canonical_current_executable();
    let first_identity = ProcessIdentity {
        pid: 12_001,
        creation_time: "first".to_owned(),
        canonical_executable_path: executable_path.clone(),
    };
    let second_identity = ProcessIdentity {
        pid: 12_002,
        creation_time: "second".to_owned(),
        canonical_executable_path: executable_path.clone(),
    };
    for response in [
        ElevatedHelperResponse {
            protocol_version: ELEVATED_HELPER_PROTOCOL_VERSION,
            nonce: "development-start".to_owned(),
            accepted: true,
            error: None,
            results: vec![ElevatedOperationResult::Launched {
                process_identity: first_identity,
            }],
        },
        ElevatedHelperResponse {
            protocol_version: ELEVATED_HELPER_PROTOCOL_VERSION,
            nonce: "development-close".to_owned(),
            accepted: true,
            error: None,
            results: vec![ElevatedOperationResult::GracefulStopRequested {
                requested: true,
                exited: true,
            }],
        },
        ElevatedHelperResponse {
            protocol_version: ELEVATED_HELPER_PROTOCOL_VERSION,
            nonce: "development-relaunch".to_owned(),
            accepted: true,
            error: None,
            results: vec![ElevatedOperationResult::Launched {
                process_identity: second_identity.clone(),
            }],
        },
    ] {
        broker.queue_response(Ok(response));
    }
    let mut core = FormationLapCore::open_with_runtime_and_privilege_broker(
        storage.path(),
        PrivilegedStartupRuntime,
        broker,
    )
    .expect("the core should open with the approved test adapters");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Elevated Restart".to_owned(),
            primary_sim_name: "Primary".to_owned(),
        })
        .expect("the profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("unexpected create outcome: {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("the created profile should be selected");
    profile.primary_sim.launch_recipe = direct_recipe(&executable_path, true);
    profile.primary_sim.path_needs_repair = false;
    let application_id = profile.primary_sim.id.clone();
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("the elevated Primary Sim should save");

    core.execute(AppCommand::StartApplication {
        profile_id: profile_id.clone(),
        application_id: application_id.clone(),
    })
    .expect("the initial elevated launch should succeed");
    assert_eq!(
        core.execute(AppCommand::RestartApplication {
            profile_id,
            application_id: application_id.clone(),
            pre_existing_confirmed: false,
        })
        .expect("the elevated application should close and relaunch"),
        CommandOutcome::ApplicationRestarted {
            application_id: application_id.clone(),
        }
    );
    assert_eq!(
        core.snapshot().application_processes[0].identity,
        Some(second_identity)
    );
    let batches = observed_broker.recorded_batches();
    assert_eq!(batches.len(), 3);
    assert!(matches!(
        batches[0].as_slice(),
        [ElevatedOperation::Launch { .. }]
    ));
    assert!(matches!(
        batches[1].as_slice(),
        [ElevatedOperation::GracefulStop { .. }]
    ));
    assert!(matches!(
        batches[2].as_slice(),
        [ElevatedOperation::Launch { .. }]
    ));
}
