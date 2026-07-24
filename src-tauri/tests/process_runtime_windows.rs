#![cfg(all(windows, feature = "process-fixtures"))]

use formation_lap_lib::{
    ConsoleVisibility, LaunchRecipe, LaunchSource, ProcessObservation, ProcessResponsiveness,
    ProcessRuntime, ShutdownStrategy, WindowsProcessRuntime,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TempDirectory {
    path: PathBuf,
}

impl TempDirectory {
    fn new() -> Self {
        let unique = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "formation-lap-runtime-test-{}-{timestamp}-{unique}",
            process::id()
        ));
        fs::create_dir_all(&path).expect("temporary runtime directory should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn wait_for_file(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !path.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(path.exists(), "{} was not written", path.display());
}

#[test]
fn direct_launch_preserves_arguments_working_directory_and_stable_identity() {
    let temporary = TempDirectory::new();
    let working_directory = temporary.path().join("working directory with spaces");
    fs::create_dir(&working_directory).expect("fixture working directory should be created");
    let report_path = temporary.path().join("argument report.json");
    let fixture_path = PathBuf::from(env!(
        "CARGO_BIN_EXE_formation-lap-process-fixture",
        "process fixture should be built with the process-fixtures feature"
    ))
    .canonicalize()
    .expect("fixture executable should canonicalize");
    let expected_arguments = vec![
        "value with spaces".to_owned(),
        "quote\"inside".to_owned(),
        "& echo shell syntax stays data".to_owned(),
    ];
    let recipe = LaunchRecipe {
        source: LaunchSource::DirectExecutable {
            executable_path: fixture_path.to_string_lossy().into_owned(),
        },
        arguments: [
            vec![
                "--report".to_owned(),
                report_path.to_string_lossy().into_owned(),
                "--lifetime-ms".to_owned(),
                "750".to_owned(),
            ],
            expected_arguments.clone(),
        ]
        .concat(),
        working_directory: Some(working_directory.to_string_lossy().into_owned()),
        monitored_process: None,
        console_visibility: ConsoleVisibility::Hidden,
        elevated: false,
        startup_timeout_seconds: 3,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::CloseWindows,
    };
    let mut runtime = WindowsProcessRuntime::new();

    let identity = runtime
        .launch(&recipe)
        .expect("the fixture should launch directly");

    assert_ne!(identity.pid, 0);
    assert!(!identity.creation_time.is_empty());
    assert_eq!(
        PathBuf::from(&identity.canonical_executable_path),
        fixture_path
    );
    assert!(
        runtime
            .matching_processes(&recipe)
            .expect("the running fixture should be observable")
            .contains(&identity)
    );
    wait_for_file(&report_path);
    let report: Value =
        serde_json::from_slice(&fs::read(&report_path).expect("fixture report should be readable"))
            .expect("fixture report should be valid JSON");
    assert_eq!(
        report["arguments"],
        serde_json::to_value(expected_arguments).expect("arguments should serialize")
    );
    assert_eq!(
        PathBuf::from(
            report["workingDirectory"]
                .as_str()
                .expect("working directory should be a string")
        ),
        working_directory
    );
}

#[test]
fn virtual_desktop_switcher_compatible_recipe_runs_as_an_ordinary_application() {
    let temporary = TempDirectory::new();
    let fixture_source = PathBuf::from(env!(
        "CARGO_BIN_EXE_formation-lap-process-fixture",
        "process fixture should be built with the process-fixtures feature"
    ));
    let executable_path = temporary.path().join("VirtualDesktopSwitcher.exe");
    fs::copy(fixture_source, &executable_path)
        .expect("VirtualDesktopSwitcher-compatible fixture should be copied");
    let report_path = temporary
        .path()
        .join("virtual desktop switcher report.json");
    let expected_arguments = vec!["--monitor".to_owned(), "primary sim".to_owned()];
    let recipe = LaunchRecipe {
        source: LaunchSource::DirectExecutable {
            executable_path: executable_path.to_string_lossy().into_owned(),
        },
        arguments: [
            vec![
                "--report".to_owned(),
                report_path.to_string_lossy().into_owned(),
                "--lifetime-ms".to_owned(),
                "5000".to_owned(),
                "--output-bytes".to_owned(),
                "128".to_owned(),
            ],
            expected_arguments.clone(),
        ]
        .concat(),
        working_directory: Some(temporary.path().to_string_lossy().into_owned()),
        monitored_process: None,
        console_visibility: ConsoleVisibility::Hidden,
        elevated: false,
        startup_timeout_seconds: 30,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::ForceOnly,
    };
    let mut runtime = WindowsProcessRuntime::new();

    let identity = runtime
        .launch(&recipe)
        .expect("VirtualDesktopSwitcher-compatible recipe should launch");
    wait_for_file(&report_path);
    let report: Value =
        serde_json::from_slice(&fs::read(&report_path).expect("report should be readable"))
            .expect("report should be valid JSON");
    assert_eq!(
        report["arguments"],
        serde_json::to_value(expected_arguments).expect("arguments should serialize")
    );
    let output = runtime
        .read_output(&identity)
        .expect("hidden console output should be captured");
    assert!(output.stdout.ends_with("STDOUT-END\n"));
    assert!(output.stderr.ends_with("STDERR-END\n"));
    runtime
        .force_stop(&identity)
        .expect("demonstration fixture should be cleaned up");
}

#[test]
fn launcher_style_launch_returns_the_monitored_process_identity() {
    let temporary = TempDirectory::new();
    let report_path = temporary.path().join("monitored report.json");
    let fixture_source = PathBuf::from(env!(
        "CARGO_BIN_EXE_formation-lap-process-fixture",
        "process fixture should be built with the process-fixtures feature"
    ));
    let monitored_path = temporary.path().join("monitored-process-fixture.exe");
    fs::copy(fixture_source, &monitored_path).expect("monitored fixture should be copied");
    let launcher_path = PathBuf::from(env!(
        "CARGO_BIN_EXE_formation-lap-launcher-fixture",
        "launcher fixture should be built with the process-fixtures feature"
    ));
    let recipe = LaunchRecipe {
        source: LaunchSource::DirectExecutable {
            executable_path: launcher_path.to_string_lossy().into_owned(),
        },
        arguments: vec![
            "--target".to_owned(),
            monitored_path.to_string_lossy().into_owned(),
            "--report".to_owned(),
            report_path.to_string_lossy().into_owned(),
            "--lifetime-ms".to_owned(),
            "750".to_owned(),
        ],
        working_directory: Some(temporary.path().to_string_lossy().into_owned()),
        monitored_process: Some("monitored-process-fixture.exe".to_owned()),
        console_visibility: ConsoleVisibility::Hidden,
        elevated: false,
        startup_timeout_seconds: 3,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::CloseWindows,
    };
    let mut runtime = WindowsProcessRuntime::new();

    let identity = runtime
        .launch(&recipe)
        .expect("the monitored child should be observed after its launcher exits");

    assert_eq!(
        PathBuf::from(&identity.canonical_executable_path),
        monitored_path
            .canonicalize()
            .expect("monitored fixture should canonicalize")
    );
    wait_for_file(&report_path);
}

#[test]
fn observation_rejects_reused_pid_metadata_and_reports_background_exit() {
    let temporary = TempDirectory::new();
    let report_path = temporary.path().join("identity report.json");
    let fixture_path = PathBuf::from(env!(
        "CARGO_BIN_EXE_formation-lap-process-fixture",
        "process fixture should be built with the process-fixtures feature"
    ));
    let recipe = LaunchRecipe {
        source: LaunchSource::DirectExecutable {
            executable_path: fixture_path.to_string_lossy().into_owned(),
        },
        arguments: vec![
            "--report".to_owned(),
            report_path.to_string_lossy().into_owned(),
            "--lifetime-ms".to_owned(),
            "500".to_owned(),
        ],
        working_directory: Some(temporary.path().to_string_lossy().into_owned()),
        monitored_process: None,
        console_visibility: ConsoleVisibility::Hidden,
        elevated: false,
        startup_timeout_seconds: 3,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::CloseWindows,
    };
    let mut runtime = WindowsProcessRuntime::new();
    let identity = runtime
        .launch(&recipe)
        .expect("identity fixture should launch");
    let mut stale_identity = identity.clone();
    stale_identity.creation_time = "0".to_owned();

    assert_eq!(
        runtime
            .observe(&stale_identity)
            .expect("the reused PID should be observed safely"),
        ProcessObservation::Replaced {
            current_identity: identity.clone(),
        }
    );
    assert_eq!(
        runtime
            .observe(&identity)
            .expect("the background fixture should be observed"),
        ProcessObservation::Running {
            responsiveness: ProcessResponsiveness::NotApplicable,
        }
    );
    wait_for_file(&report_path);
    thread::sleep(Duration::from_millis(550));
    assert_eq!(
        runtime
            .observe(&identity)
            .expect("the fixture exit should be observed"),
        ProcessObservation::Exited
    );
}

#[test]
fn slow_and_failing_fixture_modes_expose_real_process_transitions() {
    let temporary = TempDirectory::new();
    let fixture_path = PathBuf::from(env!(
        "CARGO_BIN_EXE_formation-lap-process-fixture",
        "process fixture should be built with the process-fixtures feature"
    ));
    let mut runtime = WindowsProcessRuntime::new();

    let slow_report = temporary.path().join("slow fixture.json");
    let slow_recipe = LaunchRecipe {
        source: LaunchSource::DirectExecutable {
            executable_path: fixture_path.to_string_lossy().into_owned(),
        },
        arguments: vec![
            "--report".to_owned(),
            slow_report.to_string_lossy().into_owned(),
            "--lifetime-ms".to_owned(),
            "5000".to_owned(),
            "--startup-delay-ms".to_owned(),
            "300".to_owned(),
        ],
        working_directory: Some(temporary.path().to_string_lossy().into_owned()),
        monitored_process: None,
        console_visibility: ConsoleVisibility::Hidden,
        elevated: false,
        startup_timeout_seconds: 3,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::ForceOnly,
    };
    let slow_identity = runtime
        .launch(&slow_recipe)
        .expect("slow fixture should appear before it reports readiness");
    assert!(!slow_report.exists());
    wait_for_file(&slow_report);
    assert_eq!(
        runtime
            .observe(&slow_identity)
            .expect("slow fixture should still be observable"),
        ProcessObservation::Running {
            responsiveness: ProcessResponsiveness::NotApplicable,
        }
    );
    runtime
        .force_stop(&slow_identity)
        .expect("slow fixture should be cleaned up");

    let failing_report = temporary.path().join("failing fixture.json");
    let failing_recipe = LaunchRecipe {
        arguments: vec![
            "--report".to_owned(),
            failing_report.to_string_lossy().into_owned(),
            "--lifetime-ms".to_owned(),
            "0".to_owned(),
            "--exit-code".to_owned(),
            "23".to_owned(),
        ],
        ..slow_recipe
    };
    let failing_identity = runtime
        .launch(&failing_recipe)
        .expect("failing fixture should launch before exiting");
    wait_for_file(&failing_report);
    assert!(
        runtime
            .wait_for_exit(&failing_identity, Duration::from_secs(2))
            .expect("failing fixture exit should be observed")
    );
    assert_eq!(
        runtime
            .observe(&failing_identity)
            .expect("failing fixture should remain safely distinguishable"),
        ProcessObservation::Exited
    );
}

#[test]
fn observation_distinguishes_responsive_and_hung_windowed_processes() {
    let temporary = TempDirectory::new();
    let fixture_path = PathBuf::from(env!(
        "CARGO_BIN_EXE_formation-lap-process-fixture",
        "process fixture should be built with the process-fixtures feature"
    ));
    let mut runtime = WindowsProcessRuntime::new();

    let healthy_report = temporary.path().join("healthy window.json");
    let healthy_recipe = LaunchRecipe {
        source: LaunchSource::DirectExecutable {
            executable_path: fixture_path.to_string_lossy().into_owned(),
        },
        arguments: vec![
            "--report".to_owned(),
            healthy_report.to_string_lossy().into_owned(),
            "--lifetime-ms".to_owned(),
            "1200".to_owned(),
            "--window-state".to_owned(),
            "healthy".to_owned(),
        ],
        working_directory: Some(temporary.path().to_string_lossy().into_owned()),
        monitored_process: None,
        console_visibility: ConsoleVisibility::Hidden,
        elevated: false,
        startup_timeout_seconds: 3,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::CloseWindows,
    };
    let healthy_identity = runtime
        .launch(&healthy_recipe)
        .expect("healthy window fixture should launch");
    wait_for_file(&healthy_report);
    assert_eq!(
        runtime
            .observe(&healthy_identity)
            .expect("healthy window should be observed"),
        ProcessObservation::Running {
            responsiveness: ProcessResponsiveness::Responsive,
        }
    );

    let hung_report = temporary.path().join("hung window.json");
    let hung_recipe = LaunchRecipe {
        arguments: vec![
            "--report".to_owned(),
            hung_report.to_string_lossy().into_owned(),
            "--lifetime-ms".to_owned(),
            "7000".to_owned(),
            "--window-state".to_owned(),
            "hung".to_owned(),
        ],
        ..healthy_recipe
    };
    let hung_identity = runtime
        .launch(&hung_recipe)
        .expect("hung window fixture should launch");
    wait_for_file(&hung_report);
    thread::sleep(Duration::from_millis(5_500));
    assert_eq!(
        runtime
            .observe(&hung_identity)
            .expect("hung window should be observed"),
        ProcessObservation::Running {
            responsiveness: ProcessResponsiveness::NotResponsive,
        }
    );
}

#[test]
fn close_windows_gracefully_stops_a_windowed_fixture_before_force_is_needed() {
    let temporary = TempDirectory::new();
    let report_path = temporary.path().join("graceful window.json");
    let fixture_path = PathBuf::from(env!(
        "CARGO_BIN_EXE_formation-lap-process-fixture",
        "process fixture should be built with the process-fixtures feature"
    ));
    let recipe = LaunchRecipe {
        source: LaunchSource::DirectExecutable {
            executable_path: fixture_path.to_string_lossy().into_owned(),
        },
        arguments: vec![
            "--report".to_owned(),
            report_path.to_string_lossy().into_owned(),
            "--lifetime-ms".to_owned(),
            "10000".to_owned(),
            "--window-state".to_owned(),
            "healthy".to_owned(),
        ],
        working_directory: Some(temporary.path().to_string_lossy().into_owned()),
        monitored_process: None,
        console_visibility: ConsoleVisibility::Hidden,
        elevated: false,
        startup_timeout_seconds: 3,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::CloseWindows,
    };
    let mut runtime = WindowsProcessRuntime::new();
    let identity = runtime
        .launch(&recipe)
        .expect("windowed fixture should launch");
    wait_for_file(&report_path);

    assert_eq!(
        runtime
            .request_graceful_stop(&identity, &ShutdownStrategy::CloseWindows)
            .expect("window close should be requested"),
        formation_lap_lib::GracefulStopResult::Requested
    );
    assert!(
        runtime
            .wait_for_exit(&identity, Duration::from_secs(2))
            .expect("windowed fixture exit should be observed")
    );
}

#[test]
fn custom_stop_runs_structured_arguments_and_allows_the_target_to_exit() {
    let temporary = TempDirectory::new();
    let report_path = temporary.path().join("custom stop target.json");
    let stop_file = temporary.path().join("custom stop.signal");
    let fixture_path = PathBuf::from(env!(
        "CARGO_BIN_EXE_formation-lap-process-fixture",
        "process fixture should be built with the process-fixtures feature"
    ));
    let stop_fixture_path = PathBuf::from(env!(
        "CARGO_BIN_EXE_formation-lap-stop-fixture",
        "stop fixture should be built with the process-fixtures feature"
    ));
    let recipe = LaunchRecipe {
        source: LaunchSource::DirectExecutable {
            executable_path: fixture_path.to_string_lossy().into_owned(),
        },
        arguments: vec![
            "--report".to_owned(),
            report_path.to_string_lossy().into_owned(),
            "--lifetime-ms".to_owned(),
            "10000".to_owned(),
            "--stop-file".to_owned(),
            stop_file.to_string_lossy().into_owned(),
        ],
        working_directory: Some(temporary.path().to_string_lossy().into_owned()),
        monitored_process: None,
        console_visibility: ConsoleVisibility::Hidden,
        elevated: false,
        startup_timeout_seconds: 3,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::CustomStop {
            executable_path: stop_fixture_path.to_string_lossy().into_owned(),
            arguments: vec![
                "--signal".to_owned(),
                stop_file.to_string_lossy().into_owned(),
            ],
        },
    };
    let mut runtime = WindowsProcessRuntime::new();
    let identity = runtime
        .launch(&recipe)
        .expect("custom-stop target should launch");
    wait_for_file(&report_path);

    assert_eq!(
        runtime
            .request_graceful_stop(&identity, &recipe.shutdown_strategy)
            .expect("custom stop should run"),
        formation_lap_lib::GracefulStopResult::Requested
    );
    assert!(
        runtime
            .wait_for_exit(&identity, Duration::from_secs(2))
            .expect("custom-stop target exit should be observed")
    );
}

#[test]
fn console_interrupt_gracefully_stops_a_visible_console_fixture() {
    let temporary = TempDirectory::new();
    let report_path = temporary.path().join("console interrupt target.json");
    let fixture_path = PathBuf::from(env!(
        "CARGO_BIN_EXE_formation-lap-process-fixture",
        "process fixture should be built with the process-fixtures feature"
    ));
    let recipe = LaunchRecipe {
        source: LaunchSource::DirectExecutable {
            executable_path: fixture_path.to_string_lossy().into_owned(),
        },
        arguments: vec![
            "--report".to_owned(),
            report_path.to_string_lossy().into_owned(),
            "--lifetime-ms".to_owned(),
            "10000".to_owned(),
            "--console-control".to_owned(),
        ],
        working_directory: Some(temporary.path().to_string_lossy().into_owned()),
        monitored_process: None,
        console_visibility: ConsoleVisibility::Visible,
        elevated: false,
        startup_timeout_seconds: 3,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::ConsoleInterrupt,
    };
    let mut runtime = WindowsProcessRuntime::new();
    let identity = runtime
        .launch(&recipe)
        .expect("console fixture should launch");
    wait_for_file(&report_path);

    assert_eq!(
        runtime
            .request_graceful_stop(&identity, &ShutdownStrategy::ConsoleInterrupt)
            .expect("console interrupt should be requested"),
        formation_lap_lib::GracefulStopResult::Requested
    );
    assert!(
        runtime
            .wait_for_exit(&identity, Duration::from_secs(2))
            .expect("console fixture exit should be observed")
    );
}

#[test]
fn hidden_console_output_capture_keeps_only_a_bounded_tail() {
    let temporary = TempDirectory::new();
    let report_path = temporary.path().join("bounded output target.json");
    let fixture_path = PathBuf::from(env!(
        "CARGO_BIN_EXE_formation-lap-process-fixture",
        "process fixture should be built with the process-fixtures feature"
    ));
    let recipe = LaunchRecipe {
        source: LaunchSource::DirectExecutable {
            executable_path: fixture_path.to_string_lossy().into_owned(),
        },
        arguments: vec![
            "--report".to_owned(),
            report_path.to_string_lossy().into_owned(),
            "--lifetime-ms".to_owned(),
            "1000".to_owned(),
            "--output-bytes".to_owned(),
            "70000".to_owned(),
        ],
        working_directory: Some(temporary.path().to_string_lossy().into_owned()),
        monitored_process: None,
        console_visibility: ConsoleVisibility::Hidden,
        elevated: false,
        startup_timeout_seconds: 3,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::ForceOnly,
    };
    let mut runtime = WindowsProcessRuntime::new();
    let identity = runtime
        .launch(&recipe)
        .expect("output fixture should launch");
    wait_for_file(&report_path);

    let output = runtime
        .read_output(&identity)
        .expect("captured output should be readable");

    assert!(output.truncated);
    assert!(output.stdout.len() <= 65_536);
    assert!(output.stderr.len() <= 65_536);
    assert!(output.stdout.ends_with("STDOUT-END\n"));
    assert!(output.stderr.ends_with("STDERR-END\n"));
}

#[test]
fn force_only_shutdown_requires_force_and_terminates_the_exact_identity() {
    let temporary = TempDirectory::new();
    let report_path = temporary.path().join("force-only target.json");
    let fixture_path = PathBuf::from(env!(
        "CARGO_BIN_EXE_formation-lap-process-fixture",
        "process fixture should be built with the process-fixtures feature"
    ));
    let recipe = LaunchRecipe {
        source: LaunchSource::DirectExecutable {
            executable_path: fixture_path.to_string_lossy().into_owned(),
        },
        arguments: vec![
            "--report".to_owned(),
            report_path.to_string_lossy().into_owned(),
            "--lifetime-ms".to_owned(),
            "10000".to_owned(),
        ],
        working_directory: Some(temporary.path().to_string_lossy().into_owned()),
        monitored_process: None,
        console_visibility: ConsoleVisibility::Hidden,
        elevated: false,
        startup_timeout_seconds: 3,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::ForceOnly,
    };
    let mut runtime = WindowsProcessRuntime::new();
    let identity = runtime
        .launch(&recipe)
        .expect("force-only fixture should launch");
    wait_for_file(&report_path);

    assert_eq!(
        runtime
            .request_graceful_stop(&identity, &ShutdownStrategy::ForceOnly)
            .expect("force-only shutdown should report no graceful adapter"),
        formation_lap_lib::GracefulStopResult::Unavailable
    );
    runtime
        .force_stop(&identity)
        .expect("exact fixture identity should be force stopped");
    assert!(
        runtime
            .wait_for_exit(&identity, Duration::from_secs(2))
            .expect("force-stopped fixture exit should be observed")
    );
}
