//! Formation Lap's narrow native host interface.

mod atomic_file;
mod commands;
mod contracts;
mod core;
mod discovery_catalog;
mod process_runtime;
mod profile_library;
mod settings;

pub use commands::{
    ApplicationTargetPayload, CommandError, CreateProfilePayload, DuplicateProfilePayload,
    ExitApplicationPayload, ForceStopApplicationPayload, ImportProfilePayload, NativeCommandHost,
    PrimarySimIdPayload, ProfileIdPayload, RestartApplicationPayload, SaveProfilePayload,
    create_profile, delete_profile, discover_applications, duplicate_profile, exit_application,
    export_profile, force_stop_application, get_app_snapshot, import_profile,
    recommend_applications, refresh_processes, restart_application, save_profile, select_profile,
    start_application,
};
pub use contracts::{
    AppSnapshot, ApplicationIcon, ApplicationProcessSnapshot, ApplicationRequirement,
    CatalogPrimarySim, CatalogSupportingApplication, CatalogUpdateProvider, CloseSessionSettings,
    CompatibilityRank, ConsoleVisibility, DiscoveredInstallation, DiscoveredPrimarySim,
    DiscoveredSupportingApplication, DiscoverySnapshot, LaunchRecipe, LaunchSource,
    ProcessIdentity, ProcessOutput, ProcessOwnership, ProcessStatus, ProfileApplication,
    ProfileSummary, RacingProfile, ShutdownStrategy, SupportingApplication,
    SupportingApplicationRecommendation, VrLaunchMode,
};
pub use core::{AppCommand, CommandOutcome, CoreError, FormationLapCore};
pub use discovery_catalog::{
    DiscoveryCatalogError, TargetedDiscoverySources, WindowsInstalledApplication,
    WindowsKnownLocation, WindowsKnownLocationRoot, WindowsRunningProcess,
    validate_catalog_documents,
};
pub use process_runtime::{
    GracefulStopResult, ProcessObservation, ProcessResponsiveness, ProcessRuntime,
    ProcessRuntimeError, WindowsProcessRuntime,
};
use profile_library::ProfileLibrary;
use settings::SettingsStore;
use tauri::Manager;
use tauri::Url;

fn navigation_is_allowed(url: &Url) -> bool {
    match (url.scheme(), url.host_str()) {
        ("tauri", _) => true,
        ("http" | "https", Some("tauri.localhost")) => true,
        ("http", Some("localhost" | "127.0.0.1")) if cfg!(debug_assertions) => true,
        _ => false,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let navigation_guard = tauri::plugin::Builder::<tauri::Wry>::new("navigation-guard")
        .on_navigation(|_webview, url| navigation_is_allowed(url))
        .build();

    tauri::Builder::default()
        .plugin(navigation_guard)
        .setup(|app| {
            let storage_root = app.path().app_config_dir()?;
            let commands = NativeCommandHost::open(storage_root)
                .map_err(|error| std::io::Error::other(error.message))?;
            app.manage(commands);
            Ok(())
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
            commands::start_application,
            commands::refresh_processes,
            commands::exit_application,
            commands::force_stop_application,
            commands::restart_application,
            commands::discover_applications,
            commands::recommend_applications
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
}
