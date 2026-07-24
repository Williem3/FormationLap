use formation_lap_lib::{
    AppSnapshot, ApplicationProcessSnapshot, ApplicationRequirement, ApplicationTargetPayload,
    CloseSessionSettings, CommandError, ConsoleVisibility, CreateProfilePayload,
    DuplicateProfilePayload, ExitApplicationPayload, ForceStopApplicationPayload,
    ImportProfilePayload, LaunchRecipe, LaunchSource, ProcessIdentity, ProcessOutput,
    ProcessOwnership, ProcessStatus, ProfileApplication, ProfileIdPayload, ProfileSummary,
    RacingProfile, RestartApplicationPayload, SaveProfilePayload, ShutdownStrategy,
    SupportingApplication, VrLaunchMode,
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
        LaunchSource::decl(&config),
        ShutdownStrategy::decl(&config),
        LaunchRecipe::decl(&config),
        ProfileApplication::decl(&config),
        SupportingApplication::decl(&config),
        VrLaunchMode::decl(&config),
        CloseSessionSettings::decl(&config),
        RacingProfile::decl(&config),
        ProfileSummary::decl(&config),
        ProcessIdentity::decl(&config),
        ProcessOwnership::decl(&config),
        ProcessStatus::decl(&config),
        ProcessOutput::decl(&config),
        ApplicationProcessSnapshot::decl(&config),
        AppSnapshot::decl(&config),
        CommandError::decl(&config),
        CreateProfilePayload::decl(&config),
        SaveProfilePayload::decl(&config),
        ProfileIdPayload::decl(&config),
        DuplicateProfilePayload::decl(&config),
        ImportProfilePayload::decl(&config),
        ApplicationTargetPayload::decl(&config),
        ExitApplicationPayload::decl(&config),
        ForceStopApplicationPayload::decl(&config),
        RestartApplicationPayload::decl(&config),
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
