use formation_lap_lib::{
    AppSnapshot, ApplicationIcon, ApplicationIconSnapshot, ApplicationProcessSnapshot,
    ApplicationRequirement, ApplicationTargetPayload, ApplicationUpdateSnapshot,
    ApproveProfilePayload, CatalogPrimarySim, CatalogSupportingApplication, CatalogUpdateProvider,
    CloseSessionSettings, CommandError, CompatibilityRank, ConsoleVisibility, CreateProfilePayload,
    DesktopSettings, DiagnosticEntry, DiagnosticExport, DiscoveredInstallation,
    DiscoveredPrimarySim, DiscoveredSupportingApplication, DiscoverySnapshot,
    DuplicateProfilePayload, ExitApplicationPayload, ForceStopApplicationPayload,
    GameLaunchDiagnostic, GameLaunchTarget, ImportProfilePayload, LaunchRecipe, LaunchSource,
    PrimarySimIdPayload, ProcessIdentity, ProcessOutput, ProcessOwnership, ProcessStatus,
    ProfileApplication, ProfileIdPayload, ProfileReviewStatus, ProfileSummary, QuitDisposition,
    QuitPayload, RacingProfile, RestartApplicationPayload, SaveProfilePayload,
    SessionApplicationRole, SessionApplicationSnapshot, SessionApplicationState, SessionEvent,
    SessionEventKind, SessionSnapshot, SessionState, SessionSummary, ShutdownStrategy,
    SteamLaunchSelector, SupportingApplication, SupportingApplicationRecommendation,
    ThemePreference, UpdateChannel, UpdateSettingsPayload, UpdateSnapshot, UpdateStatus,
    VrLaunchMode,
};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::ExitCode,
};
use ts_rs::TS;

fn render_bindings() -> String {
    let config = ts_rs::Config::default();
    let declarations = [
        ApplicationRequirement::decl(&config),
        ConsoleVisibility::decl(&config),
        SteamLaunchSelector::decl(&config),
        LaunchSource::decl(&config),
        ShutdownStrategy::decl(&config),
        LaunchRecipe::decl(&config),
        GameLaunchTarget::decl(&config),
        GameLaunchDiagnostic::decl(&config),
        ProfileApplication::decl(&config),
        SupportingApplication::decl(&config),
        VrLaunchMode::decl(&config),
        CloseSessionSettings::decl(&config),
        RacingProfile::decl(&config),
        ProfileReviewStatus::decl(&config),
        ProfileSummary::decl(&config),
        ProcessIdentity::decl(&config),
        ProcessOwnership::decl(&config),
        ProcessStatus::decl(&config),
        ProcessOutput::decl(&config),
        ApplicationProcessSnapshot::decl(&config),
        SessionState::decl(&config),
        SessionApplicationRole::decl(&config),
        SessionApplicationState::decl(&config),
        SessionEventKind::decl(&config),
        SessionEvent::decl(&config),
        SessionSummary::decl(&config),
        SessionApplicationSnapshot::decl(&config),
        SessionSnapshot::decl(&config),
        ThemePreference::decl(&config),
        UpdateChannel::decl(&config),
        DesktopSettings::decl(&config),
        DiagnosticEntry::decl(&config),
        DiagnosticExport::decl(&config),
        UpdateStatus::decl(&config),
        ApplicationUpdateSnapshot::decl(&config),
        UpdateSnapshot::decl(&config),
        AppSnapshot::decl(&config),
        ApplicationIcon::decl(&config),
        ApplicationIconSnapshot::decl(&config),
        DiscoveredInstallation::decl(&config),
        CatalogPrimarySim::decl(&config),
        CatalogSupportingApplication::decl(&config),
        DiscoveredPrimarySim::decl(&config),
        DiscoveredSupportingApplication::decl(&config),
        DiscoverySnapshot::decl(&config),
        CompatibilityRank::decl(&config),
        CatalogUpdateProvider::decl(&config),
        SupportingApplicationRecommendation::decl(&config),
        CommandError::decl(&config),
        CreateProfilePayload::decl(&config),
        SaveProfilePayload::decl(&config),
        ProfileIdPayload::decl(&config),
        DuplicateProfilePayload::decl(&config),
        ImportProfilePayload::decl(&config),
        ApproveProfilePayload::decl(&config),
        ApplicationTargetPayload::decl(&config),
        ExitApplicationPayload::decl(&config),
        ForceStopApplicationPayload::decl(&config),
        RestartApplicationPayload::decl(&config),
        PrimarySimIdPayload::decl(&config),
        QuitDisposition::decl(&config),
        QuitPayload::decl(&config),
        UpdateSettingsPayload::decl(&config),
    ]
    .into_iter()
    .map(|declaration| format!("export {declaration}"))
    .collect::<Vec<_>>()
    .join("\n\n");

    format!(
        r#"// This file is generated from Rust. Do not edit by hand.
import {{ invoke }} from "@tauri-apps/api/core";

{declarations}

export function getAppSnapshot(): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("get_app_snapshot");
}}

export function createProfile(payload: CreateProfilePayload): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("create_profile", {{ payload }});
}}

export function saveProfile(payload: SaveProfilePayload): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("save_profile", {{ payload }});
}}

export function selectProfile(payload: ProfileIdPayload): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("select_profile", {{ payload }});
}}

export function duplicateProfile(payload: DuplicateProfilePayload): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("duplicate_profile", {{ payload }});
}}

export function deleteProfile(payload: ProfileIdPayload): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("delete_profile", {{ payload }});
}}

export function exportProfile(payload: ProfileIdPayload): Promise<string> {{
  return invoke<string>("export_profile", {{ payload }});
}}

export function importProfile(payload: ImportProfilePayload): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("import_profile", {{ payload }});
}}

export function approveProfile(payload: ApproveProfilePayload): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("approve_profile", {{ payload }});
}}

export function pickExecutablePath(initialPath?: string | null): Promise<string | null> {{
  return invoke<string | null>("pick_executable_path", {{ initialPath }});
}}

export function startApplication(payload: ApplicationTargetPayload): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("start_application", {{ payload }});
}}

export function refreshProcesses(): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("refresh_processes");
}}

export function exitApplication(payload: ExitApplicationPayload): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("exit_application", {{ payload }});
}}

export function forceStopApplication(payload: ForceStopApplicationPayload): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("force_stop_application", {{ payload }});
}}

export function restartApplication(payload: RestartApplicationPayload): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("restart_application", {{ payload }});
}}

export function startSession(payload: ProfileIdPayload): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("start_session", {{ payload }});
}}

export function testGameLaunch(payload: ProfileIdPayload): Promise<GameLaunchDiagnostic> {{
  return invoke<GameLaunchDiagnostic>("test_game_launch", {{ payload }});
}}

export function cancelStartup(): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("cancel_startup");
}}

export function closeSession(): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("close_session");
}}

export function requestQuit(payload: QuitPayload): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("request_quit", {{ payload }});
}}

export function updateSettings(payload: UpdateSettingsPayload): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("update_settings", {{ payload }});
}}

export function checkUpdates(): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("check_updates");
}}

export function installFormationLapUpdate(): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("install_formation_lap_update");
}}

export function exportDiagnostics(): Promise<DiagnosticExport> {{
  return invoke<DiagnosticExport>("export_diagnostics");
}}

export function acceptRecovery(): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("accept_recovery");
}}

export function dismissRecovery(): Promise<AppSnapshot> {{
  return invoke<AppSnapshot>("dismiss_recovery");
}}

export function discoverApplications(): Promise<DiscoverySnapshot> {{
  return invoke<DiscoverySnapshot>("discover_applications");
}}

export function recommendApplications(payload: PrimarySimIdPayload): Promise<SupportingApplicationRecommendation[]> {{
  return invoke<SupportingApplicationRecommendation[]>("recommend_applications", {{ payload }});
}}
"#,
    )
}

fn bindings_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("src")
        .join("generated")
        .join("bindings.ts")
}

fn write_bindings(path: &Path, expected: &str) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("generated bindings path has no parent"))?;
    fs::create_dir_all(parent)?;
    fs::write(path, expected)
}

fn check_bindings(path: &Path, expected: &str) -> io::Result<()> {
    let actual = fs::read_to_string(path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "{} is missing; run `pnpm contracts:generate`: {error}",
                path.display()
            ),
        )
    })?;

    if actual == expected {
        Ok(())
    } else {
        Err(io::Error::other(
            "generated bindings are stale; run `pnpm contracts:generate`",
        ))
    }
}

fn execute() -> Result<(), Box<dyn std::error::Error>> {
    let mode = env::args()
        .nth(1)
        .ok_or("expected exactly one mode: `--write` to regenerate or `--check` to verify")?;
    if env::args().nth(2).is_some() {
        return Err("expected exactly one mode argument".into());
    }

    let output = render_bindings();
    let path = bindings_path();

    match mode.as_str() {
        "--write" => write_bindings(&path, &output)?,
        "--check" => check_bindings(&path, &output)?,
        _ => return Err(format!("unsupported mode `{mode}`").into()),
    }

    println!("generated bindings {}: {}", mode, path.display());
    Ok(())
}

fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
