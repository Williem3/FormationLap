//! Formation Lap's narrow native host interface.

mod atomic_file;
mod commands;
mod contracts;
mod core;
mod desktop_host;
mod diagnostics;
mod discovery_catalog;
mod game_launch_diagnostics;
mod launch_recipe;
mod native_file_picker;
mod native_updater;
mod privilege_broker;
mod privilege_protocol;
mod process_runtime;
mod profile_library;
mod release_identity;
mod session_journal;
mod settings;
mod storage_migration;
mod update_advisor;
mod update_coordinator;
mod update_providers;

pub use commands::{
    ApplicationTargetPayload, ApproveProfilePayload, CommandError, CreateProfilePayload,
    DuplicateProfilePayload, ExitApplicationPayload, ForceStopApplicationPayload,
    ImportProfilePayload, NativeCommandHost, PrimarySimIdPayload, ProfileIdPayload, QuitPayload,
    RestartApplicationPayload, SaveProfilePayload, UpdateSettingsPayload, accept_recovery,
    approve_profile, cancel_startup, close_session, create_profile, delete_profile,
    discover_applications, dismiss_recovery, duplicate_profile, exit_application,
    export_diagnostics, export_profile, force_stop_application, get_app_snapshot, import_profile,
    pick_executable_path, recommend_applications, refresh_processes, request_quit,
    restart_application, save_profile, select_profile, start_application, test_game_launch,
    update_settings,
};
pub use contracts::{
    AppSnapshot, ApplicationIcon, ApplicationIconSnapshot, ApplicationProcessSnapshot,
    ApplicationRequirement, ApplicationUpdateSnapshot, ApplicationUpdateTarget, CatalogPrimarySim,
    CatalogSupportingApplication, CatalogUpdateProvider, CloseSessionSettings, CompatibilityRank,
    ConsoleVisibility, DesktopSettings, DiagnosticEntry, DiagnosticExport, DiscoveredInstallation,
    DiscoveredPrimarySim, DiscoveredSupportingApplication, DiscoverySnapshot,
    FormationLapInstallDecision, GameLaunchDiagnostic, GameLaunchTarget, LaunchRecipe,
    LaunchSource, ProcessIdentity, ProcessOutput, ProcessOwnership, ProcessStatus,
    ProfileApplication, ProfileReviewStatus, ProfileSummary, QuitAction, QuitDisposition,
    RacingProfile, SessionApplicationRole, SessionApplicationSnapshot, SessionApplicationState,
    SessionEvent, SessionEventKind, SessionSnapshot, SessionState, SessionSummary,
    ShutdownStrategy, SteamLaunchSelector, SupportingApplication,
    SupportingApplicationProfileDefaults, SupportingApplicationRecommendation, ThemePreference,
    UpdateChannel, UpdateCheckDecision, UpdateCheckPlan, UpdateCheckResult, UpdateCheckTrigger,
    UpdateSnapshot, UpdateStatus, VrLaunchMode, WindowCloseAction,
};
pub use core::{AppCommand, CommandOutcome, CoreError, FormationLapCore};
pub use discovery_catalog::{
    DiscoveryCatalogError, TargetedDiscoverySources, WindowsInstalledApplication,
    WindowsKnownLocation, WindowsKnownLocationRoot, WindowsRunningProcess,
    validate_catalog_documents,
};
pub(crate) use native_updater::FormationLapUpdater;
#[cfg(feature = "process-fixtures")]
#[doc(hidden)]
pub use privilege_broker::run_elevated_helper_for_test;
pub use privilege_broker::{
    DevelopmentPrivilegeBroker, PrivilegeBroker, PrivilegeBrokerError, WindowsPrivilegeBroker,
    run_elevated_helper,
};
pub use privilege_protocol::{
    ELEVATED_HELPER_PROTOCOL_VERSION, ElevatedHelperRequest, ElevatedHelperResponse,
    ElevatedOperation, ElevatedOperationResult, ElevatedOwnershipAcknowledgement,
    ElevatedOwnershipOffer, ElevatedRequestValidator, HelperProtocolError, HelperValidationContext,
    MAX_ELEVATED_ARGUMENT_BYTES, MAX_ELEVATED_ARGUMENTS, MAX_ELEVATED_OPERATIONS,
    MAX_HELPER_MESSAGE_BYTES, decode_helper_request, encode_helper_message,
};
pub use process_runtime::{
    GracefulStopResult, ProcessObservation, ProcessResponsiveness, ProcessRuntime,
    ProcessRuntimeError, WindowsProcessRuntime,
};
use profile_library::ProfileLibrary;
use settings::SettingsStore;
use tauri::{
    Emitter, Manager, Url,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use update_coordinator::UpdateCoordinator;
pub(crate) use update_providers::{DirectUpdateProviderRuntime, UpdateProviderRunner};

fn navigation_is_allowed(url: &Url) -> bool {
    match (url.scheme(), url.host_str()) {
        ("tauri", _) => true,
        ("http" | "https", Some("tauri.localhost")) => true,
        ("http", Some("localhost" | "127.0.0.1")) if cfg!(debug_assertions) => true,
        _ => false,
    }
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn tray_status(session: &SessionSnapshot) -> (&'static str, &'static str, bool) {
    match session.state {
        SessionState::Idle => ("Ready", "No Active Session", false),
        SessionState::Starting => ("Starting Session", "Cancel Startup", true),
        SessionState::Cancelling => ("Cancelling Startup", "Cancelling…", false),
        SessionState::Active => ("Session active", "Close Session", true),
        SessionState::Closing => ("Closing Session", "Closing…", false),
        SessionState::RecoveryAvailable => ("Recovery available", "Review Recovery", true),
    }
}

fn build_tray(
    app: &mut tauri::App,
    commands: NativeCommandHost,
    update_coordinator: UpdateCoordinator,
) -> tauri::Result<tauri::tray::TrayIcon> {
    let snapshot = commands
        .get_app_snapshot()
        .map_err(|error| std::io::Error::other(error.message))?;
    let (status_text, session_action_text, session_action_enabled) = tray_status(&snapshot.session);
    let status = MenuItem::with_id(
        app,
        "tray-status",
        format!("Status: {status_text}"),
        false,
        None::<&str>,
    )?;
    let open = MenuItem::with_id(app, "tray-open", "Open Formation Lap", true, None::<&str>)?;
    let session_action = MenuItem::with_id(
        app,
        "tray-session-action",
        session_action_text,
        session_action_enabled,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "tray-quit", "Quit…", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&status, &open, &session_action, &separator, &quit])?;

    let menu_commands = commands.clone();
    let mut builder = TrayIconBuilder::with_id("formation-lap-tray")
        .menu(&menu)
        .tooltip(format!("Formation Lap — {status_text}"))
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "tray-open" => show_main_window(app),
            "tray-session-action" => {
                let Ok(snapshot) = menu_commands.get_app_snapshot() else {
                    return;
                };
                match snapshot.session.state {
                    SessionState::Starting => {
                        let _ = menu_commands.cancel_startup();
                    }
                    SessionState::Active => {
                        let _ = menu_commands.close_session();
                    }
                    SessionState::RecoveryAvailable => show_main_window(app),
                    SessionState::Idle | SessionState::Cancelling | SessionState::Closing => {}
                }
            }
            "tray-quit" => {
                show_main_window(app);
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("formation-lap://quit-requested", ());
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } | TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    let tray = builder.build(app)?;

    let status_commands = commands;
    let status_item = status;
    let session_action_item = session_action;
    let app_handle = app.handle().clone();
    std::thread::Builder::new()
        .name("formation-lap-tray-status".to_owned())
        .spawn(move || {
            let mut last_update_attempt =
                std::time::Instant::now() - std::time::Duration::from_secs(60);
            while let Ok(snapshot) = status_commands.refresh_processes() {
                let (status_text, session_action_text, session_action_enabled) =
                    tray_status(&snapshot.session);
                let _ = status_item.set_text(format!("Status: {status_text}"));
                let _ = session_action_item.set_text(session_action_text);
                let _ = session_action_item.set_enabled(session_action_enabled);
                if let Some(tray) = app_handle.tray_by_id("formation-lap-tray") {
                    let _ = tray.set_tooltip(Some(format!("Formation Lap — {status_text}")));
                }
                if last_update_attempt.elapsed() >= std::time::Duration::from_secs(60) {
                    last_update_attempt = std::time::Instant::now();
                    let update_app = app_handle.clone();
                    let update_commands = status_commands.clone();
                    let update_coordinator = update_coordinator.clone();
                    tauri::async_runtime::spawn(async move {
                        let updater = update_app.state::<FormationLapUpdater>();
                        let _ = commands::perform_update_check(
                            &update_commands,
                            updater.inner(),
                            &update_coordinator,
                            UpdateCheckTrigger::Automatic,
                        )
                        .await;
                    });
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        })
        .map_err(tauri::Error::Io)?;

    Ok(tray)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let Ok(Some(_single_instance)) = desktop_host::SingleInstanceGuard::acquire() else {
        return;
    };
    let navigation_guard = tauri::plugin::Builder::<tauri::Wry>::new("navigation-guard")
        .on_navigation(|_webview, url| navigation_is_allowed(url))
        .build();

    tauri::Builder::default()
        .plugin(navigation_guard)
        .setup(|app| {
            let storage_root = app.path().app_local_data_dir()?;
            let roaming_storage_root = app.path().app_config_dir()?;
            let commands =
                NativeCommandHost::open_with_roaming_migration(storage_root, roaming_storage_root)
                    .map_err(|error| std::io::Error::other(error.message))?;
            let settings = commands
                .get_app_snapshot()
                .map_err(|error| std::io::Error::other(error.message))?
                .settings;
            let _ = desktop_host::set_start_with_windows(settings.start_with_windows);
            let update_coordinator = UpdateCoordinator::new();
            app.manage(commands.clone());
            app.manage(FormationLapUpdater::from_compile_time());
            app.manage(update_coordinator.clone());
            let tray = build_tray(app, commands.clone(), update_coordinator)?;
            app.manage(tray);
            if desktop_host::started_minimized()
                && let Some(window) = app.get_webview_window("main")
            {
                window.hide()?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let commands = window.state::<NativeCommandHost>();
                match commands.request_window_close() {
                    Ok(WindowCloseAction::HideToTray) => {
                        let _ = window.hide();
                    }
                    Ok(WindowCloseAction::Exit) => window.app_handle().exit(0),
                    Err(_) => {}
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_snapshot,
            commands::create_profile,
            commands::save_profile,
            commands::select_profile,
            commands::duplicate_profile,
            commands::delete_profile,
            commands::export_profile,
            commands::import_profile,
            commands::approve_profile,
            commands::pick_executable_path,
            commands::start_application,
            commands::refresh_processes,
            commands::exit_application,
            commands::force_stop_application,
            commands::restart_application,
            commands::start_session,
            commands::test_game_launch,
            commands::cancel_startup,
            commands::close_session,
            commands::request_quit,
            commands::update_settings,
            commands::export_diagnostics,
            commands::accept_recovery,
            commands::dismiss_recovery,
            commands::discover_applications,
            commands::recommend_applications,
            commands::check_updates,
            commands::install_formation_lap_update
        ])
        .run(tauri::generate_context!())
        .expect("Formation Lap failed to start");
}

#[cfg(test)]
mod tests {
    use super::navigation_is_allowed;
    use tauri::Url;

    #[test]
    fn remote_navigation_is_denied() {
        let remote = Url::parse("https://example.com").expect("valid test URL");

        assert!(!navigation_is_allowed(&remote));
    }

    #[test]
    fn bundled_windows_origin_is_allowed() {
        let bundled = Url::parse("http://tauri.localhost").expect("valid test URL");

        assert!(navigation_is_allowed(&bundled));
    }

    #[test]
    fn main_application_manifest_explicitly_remains_non_administrative() {
        let manifest = include_str!("../windows-app-manifest.xml");

        assert!(manifest.contains(r#"requestedExecutionLevel level="asInvoker""#));
        assert!(!manifest.contains("requireAdministrator"));
    }
}
