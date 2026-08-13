use formation_lap_lib::{
    AppCommand, ApplicationRequirement, CommandOutcome, ConsoleVisibility, CoreError,
    FormationLapCore, GameLaunchTarget, GracefulStopResult, LaunchRecipe, LaunchSource,
    ProcessIdentity, ProcessObservation, ProcessOutput, ProcessRuntime, ProcessRuntimeError,
    ShutdownStrategy, SteamLaunchSelector, SupportingApplication, VrLaunchMode,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::{
        Arc, Mutex,
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
            "formation-lap-launch-recipe-test-{}-{timestamp}-{unique}",
            process::id()
        ));
        fs::create_dir_all(&path).expect("temporary recipe storage should be created");
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

fn temporary_safe_executable(name: &str) -> PathBuf {
    let unique = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("formation-lap-{unique}-{name}"));
    fs::write(&path, b"test executable bytes").expect("temporary executable should be written");
    path.canonicalize()
        .expect("temporary executable should canonicalize")
}

struct RecipeRecordingRuntime {
    launched_recipes: Arc<Mutex<Vec<LaunchRecipe>>>,
    identity: ProcessIdentity,
}

impl ProcessRuntime for RecipeRecordingRuntime {
    fn matching_processes(
        &mut self,
        _recipe: &LaunchRecipe,
    ) -> Result<Vec<ProcessIdentity>, ProcessRuntimeError> {
        Ok(Vec::new())
    }

    fn launch(&mut self, recipe: &LaunchRecipe) -> Result<ProcessIdentity, ProcessRuntimeError> {
        self.launched_recipes
            .lock()
            .expect("recipe trace should not be poisoned")
            .push(recipe.clone());
        Ok(self.identity.clone())
    }

    fn observe(
        &mut self,
        _identity: &ProcessIdentity,
    ) -> Result<ProcessObservation, ProcessRuntimeError> {
        Ok(ProcessObservation::Running {
            responsiveness: formation_lap_lib::ProcessResponsiveness::NotApplicable,
        })
    }

    fn request_graceful_stop(
        &mut self,
        _identity: &ProcessIdentity,
        _strategy: &ShutdownStrategy,
    ) -> Result<GracefulStopResult, ProcessRuntimeError> {
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

#[test]
fn an_ordinary_steam_session_uses_the_curated_no_dialog_recipe() {
    let recipes = launch_steam_profile("Le Mans Ultimate", 2_399_420, false, None);

    assert_eq!(recipes.len(), 1);
    assert_eq!(
        recipes[0].source,
        LaunchSource::Steam {
            app_id: 2_399_420,
            selector: Some(SteamLaunchSelector::Default),
        }
    );
    assert_eq!(
        recipes[0].monitored_process.as_deref(),
        Some("Le Mans Ultimate.exe")
    );
}

#[test]
fn a_vr_session_selects_the_profiles_curated_vr_recipe() {
    let recipes = launch_steam_profile(
        "Le Mans Ultimate",
        2_399_420,
        true,
        Some(VrLaunchMode::OpenXr),
    );

    assert_eq!(recipes.len(), 1);
    assert!(recipes[0].arguments.is_empty());
    assert_eq!(
        recipes[0].source,
        LaunchSource::Steam {
            app_id: 2_399_420,
            selector: Some(SteamLaunchSelector::Option { index: 3 }),
        }
    );
}

#[test]
fn le_mans_ultimate_defaults_vr_to_the_curated_openxr_recipe() {
    let recipes = launch_steam_profile("Le Mans Ultimate", 2_399_420, true, None);

    assert_eq!(recipes.len(), 1);
    assert_eq!(
        recipes[0].source,
        LaunchSource::Steam {
            app_id: 2_399_420,
            selector: Some(SteamLaunchSelector::Option { index: 3 }),
        }
    );
}

#[test]
fn le_mans_ultimate_preserves_steamvr_as_an_explicit_fallback() {
    let recipes = try_launch_steam_profile(
        "Le Mans Ultimate",
        2_399_420,
        true,
        Some(VrLaunchMode::OpenVr),
        None,
        true,
    )
    .expect("Le Mans Ultimate should retain its SteamVR launch recipe");

    assert_eq!(recipes.len(), 1);
    assert_eq!(
        recipes[0].source,
        LaunchSource::Steam {
            app_id: 2_399_420,
            selector: Some(SteamLaunchSelector::OpenVr),
        }
    );
}

#[test]
fn a_curated_sim_rejects_a_vr_mode_it_does_not_declare() {
    let error = try_launch_steam_profile(
        "Assetto Corsa",
        244_210,
        true,
        Some(VrLaunchMode::OpenXr),
        None,
        true,
    )
    .expect_err("Assetto Corsa does not declare an OpenXR recipe");

    assert!(matches!(
        error,
        CoreError::InvalidLaunchRecipe(message)
            if message.contains("does not support the selected VR Launch Mode")
    ));
}

#[test]
fn a_profile_launch_recipe_overrides_curated_defaults() {
    let override_recipe = LaunchRecipe {
        source: LaunchSource::Steam {
            app_id: 211_500,
            selector: Some(SteamLaunchSelector::Default),
        },
        arguments: vec!["-user-choice".to_owned()],
        working_directory: None,
        monitored_process: Some("CustomRaceRoom.exe".to_owned()),
        monitored_executable_path: None,
        console_visibility: ConsoleVisibility::Hidden,
        elevated: false,
        startup_timeout_seconds: 30,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::CloseWindows,
    };

    let recipes = try_launch_steam_profile(
        "RaceRoom Racing Experience",
        211_500,
        false,
        None,
        Some(override_recipe.clone()),
        true,
    )
    .expect("the explicit per-profile recipe should launch");

    assert_eq!(recipes, vec![override_recipe]);
}

#[test]
fn standalone_iracing_keeps_its_direct_executable_recipe() {
    let executable = temporary_safe_executable("iRacingSim64DX11.exe");
    let working_directory = executable
        .parent()
        .expect("temporary executable should have a parent")
        .to_string_lossy()
        .to_string();
    let direct_recipe = LaunchRecipe {
        source: LaunchSource::DirectExecutable {
            executable_path: executable.to_string_lossy().to_string(),
        },
        arguments: vec!["-hosted".to_owned()],
        working_directory: Some(working_directory),
        monitored_process: None,
        monitored_executable_path: None,
        console_visibility: ConsoleVisibility::Hidden,
        elevated: false,
        startup_timeout_seconds: 30,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::CloseWindows,
    };

    let recipes =
        try_launch_steam_profile("iRacing", 0, false, None, Some(direct_recipe.clone()), true)
            .expect("standalone iRacing should launch without Steam");

    assert_eq!(recipes, vec![direct_recipe]);

    fs::remove_file(executable).expect("temporary executable should be removed");
}

#[test]
fn manual_primary_sim_start_uses_the_curated_recipe() {
    let recipes = try_launch_steam_profile("Le Mans Ultimate", 2_399_420, false, None, None, false)
        .expect("manual Primary Sim start should use the curated recipe");

    assert_eq!(
        recipes[0].source,
        LaunchSource::Steam {
            app_id: 2_399_420,
            selector: Some(SteamLaunchSelector::Default),
        }
    );
}

#[test]
fn every_curated_steam_sim_resolves_each_declared_ordinary_and_vr_recipe() {
    let cases = [
        (
            "iRacing",
            266_410,
            None,
            SteamLaunchSelector::Default,
            &[][..],
            "iRacingSim64DX11.exe",
        ),
        (
            "iRacing",
            266_410,
            Some(VrLaunchMode::OpenXr),
            SteamLaunchSelector::Default,
            &[][..],
            "iRacingSim64DX11.exe",
        ),
        (
            "iRacing",
            266_410,
            Some(VrLaunchMode::OpenVr),
            SteamLaunchSelector::Default,
            &[][..],
            "iRacingSim64DX11.exe",
        ),
        (
            "iRacing",
            266_410,
            Some(VrLaunchMode::Oculus),
            SteamLaunchSelector::Default,
            &[][..],
            "iRacingSim64DX11.exe",
        ),
        (
            "Assetto Corsa",
            244_210,
            None,
            SteamLaunchSelector::Default,
            &[][..],
            "AssettoCorsa.exe",
        ),
        (
            "Assetto Corsa",
            244_210,
            Some(VrLaunchMode::OpenVr),
            SteamLaunchSelector::Default,
            &[][..],
            "AssettoCorsa.exe",
        ),
        (
            "Assetto Corsa Competizione",
            805_550,
            None,
            SteamLaunchSelector::Default,
            &[][..],
            "acc.exe",
        ),
        (
            "Assetto Corsa Competizione",
            805_550,
            Some(VrLaunchMode::OpenVr),
            SteamLaunchSelector::OpenVr,
            &[][..],
            "acc.exe",
        ),
        (
            "Assetto Corsa Competizione",
            805_550,
            Some(VrLaunchMode::Oculus),
            SteamLaunchSelector::Oculus,
            &[][..],
            "acc.exe",
        ),
        (
            "Assetto Corsa EVO",
            3_058_630,
            None,
            SteamLaunchSelector::Default,
            &[][..],
            "AssettoCorsaEVO.exe",
        ),
        (
            "Assetto Corsa EVO",
            3_058_630,
            Some(VrLaunchMode::OpenXr),
            SteamLaunchSelector::Default,
            &["-vr"][..],
            "AssettoCorsaEVO.exe",
        ),
        (
            "Automobilista 2",
            1_066_890,
            None,
            SteamLaunchSelector::Default,
            &["-novr"][..],
            "AMS2.exe",
        ),
        (
            "Automobilista 2",
            1_066_890,
            Some(VrLaunchMode::OpenVr),
            SteamLaunchSelector::OpenVr,
            &[][..],
            "AMS2.exe",
        ),
        (
            "Automobilista 2",
            1_066_890,
            Some(VrLaunchMode::Oculus),
            SteamLaunchSelector::Oculus,
            &[][..],
            "AMS2.exe",
        ),
        (
            "rFactor 2",
            365_960,
            None,
            SteamLaunchSelector::Default,
            &[][..],
            "rFactor2.exe",
        ),
        (
            "rFactor 2",
            365_960,
            Some(VrLaunchMode::OpenVr),
            SteamLaunchSelector::OpenVr,
            &[][..],
            "rFactor2.exe",
        ),
        (
            "Le Mans Ultimate",
            2_399_420,
            None,
            SteamLaunchSelector::Default,
            &[][..],
            "Le Mans Ultimate.exe",
        ),
        (
            "Le Mans Ultimate",
            2_399_420,
            Some(VrLaunchMode::OpenVr),
            SteamLaunchSelector::OpenVr,
            &[][..],
            "Le Mans Ultimate.exe",
        ),
        (
            "Le Mans Ultimate",
            2_399_420,
            Some(VrLaunchMode::OpenXr),
            SteamLaunchSelector::Option { index: 3 },
            &[][..],
            "Le Mans Ultimate.exe",
        ),
        (
            "RaceRoom Racing Experience",
            211_500,
            None,
            SteamLaunchSelector::Option { index: 1 },
            &[][..],
            "RRRE64.exe",
        ),
        (
            "RaceRoom Racing Experience",
            211_500,
            Some(VrLaunchMode::OpenVr),
            SteamLaunchSelector::OpenVr,
            &[][..],
            "RRRE64.exe",
        ),
        (
            "EA SPORTS WRC",
            1_849_250,
            None,
            SteamLaunchSelector::Default,
            &[][..],
            "WRC.exe",
        ),
        (
            "EA SPORTS WRC",
            1_849_250,
            Some(VrLaunchMode::OpenXr),
            SteamLaunchSelector::Option { index: 2 },
            &[][..],
            "WRC.exe",
        ),
        (
            "DiRT Rally 2.0",
            690_790,
            None,
            SteamLaunchSelector::Default,
            &["-novr"][..],
            "dirtrally2.exe",
        ),
        (
            "DiRT Rally 2.0",
            690_790,
            Some(VrLaunchMode::OpenVr),
            SteamLaunchSelector::OpenVr,
            &[][..],
            "dirtrally2.exe",
        ),
        (
            "DiRT Rally 2.0",
            690_790,
            Some(VrLaunchMode::Oculus),
            SteamLaunchSelector::Oculus,
            &[][..],
            "dirtrally2.exe",
        ),
    ];

    for (name, app_id, vr_mode, selector, arguments, monitored_process) in cases {
        let recipes = launch_steam_profile(name, app_id, vr_mode.is_some(), vr_mode);
        assert_eq!(recipes.len(), 1, "{name}");
        assert_eq!(
            recipes[0].source,
            LaunchSource::Steam {
                app_id,
                selector: Some(selector),
            },
            "{name}"
        );
        assert_eq!(recipes[0].arguments, arguments, "{name}");
        assert_eq!(
            recipes[0].monitored_process.as_deref(),
            Some(monitored_process),
            "{name}"
        );
    }
}

#[test]
fn test_game_launch_starts_only_the_primary_sim_and_writes_a_sanitized_report() {
    let storage = TempStorage::new();
    let launched_recipes = Arc::new(Mutex::new(Vec::new()));
    let runtime = RecipeRecordingRuntime {
        launched_recipes: Arc::clone(&launched_recipes),
        identity: ProcessIdentity {
            pid: 7_007,
            creation_time: "133822944900000001".to_owned(),
            canonical_executable_path: r"C:\Private Games\LMU\Le Mans Ultimate.exe".to_owned(),
        },
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("core should open with the recipe recorder");
    let CommandOutcome::ProfileCreated { profile_id } = core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Le Mans Ultimate".to_owned(),
                "Le Mans Ultimate".to_owned(),
            )),
        })
        .expect("profile should be created")
    else {
        panic!("create should return the new profile id");
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("new profile should be selected");
    profile.primary_sim.launch_recipe = LaunchRecipe {
        source: LaunchSource::Steam {
            app_id: 2_399_420,
            selector: None,
        },
        arguments: Vec::new(),
        working_directory: None,
        monitored_process: None,
        monitored_executable_path: None,
        console_visibility: ConsoleVisibility::Hidden,
        elevated: false,
        startup_timeout_seconds: 30,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::CloseWindows,
    };
    profile.primary_sim.path_needs_repair = false;
    profile.supporting_applications = vec![SupportingApplication {
        application: formation_lap_lib::ProfileApplication {
            id: "support-fixture".to_owned(),
            name: "Support fixture".to_owned(),
            launch_recipe: LaunchRecipe::default(),
            path_needs_repair: true,
        },
        requirement: ApplicationRequirement::Required,
        keep_running: false,
    }];
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("profile should save");
    let catalog_before =
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../catalog/sims.json"))
            .expect("bundled sim catalog should be readable");

    let CommandOutcome::GameLaunchTested { diagnostic } = core
        .execute(AppCommand::TestGameLaunch {
            profile_id: profile_id.clone(),
        })
        .expect("Test Game Launch should succeed")
    else {
        panic!("test launch should return its diagnostic");
    };

    assert_eq!(launched_recipes.lock().unwrap().len(), 1);
    assert_eq!(
        diagnostic.target,
        GameLaunchTarget::Steam {
            uri: "steam://launch/2399420/option0".to_owned(),
        }
    );
    assert_eq!(diagnostic.observed_process, "Le Mans Ultimate.exe");
    let report = fs::read_to_string(storage.path().join("logs/test-game-launch.json"))
        .expect("the local diagnostic report should be written");
    assert!(report.contains("steam://launch/2399420/option0"));
    assert!(report.contains("Le Mans Ultimate.exe"));
    assert!(!report.contains(r"C:\Private Games"));
    assert_eq!(
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../catalog/sims.json"))
            .expect("bundled sim catalog should remain readable"),
        catalog_before
    );
    assert_eq!(
        core.snapshot().session.state,
        formation_lap_lib::SessionState::Idle
    );
    assert_eq!(
        core.snapshot()
            .selected_profile
            .expect("tested profile should remain selected")
            .primary_sim
            .launch_recipe
            .monitored_process
            .as_deref(),
        Some("Le Mans Ultimate.exe"),
        "the observed Process name should become a per-profile override"
    );
    assert_eq!(
        core.snapshot()
            .selected_profile
            .expect("tested profile should remain selected")
            .primary_sim
            .launch_recipe
            .monitored_executable_path
            .as_deref(),
        Some(r"C:\Private Games\LMU\Le Mans Ultimate.exe"),
        "the observed canonical path should become a per-profile review candidate"
    );
}

fn launch_steam_profile(
    sim_name: &str,
    app_id: u32,
    vr_enabled: bool,
    preferred_vr_launch_mode: Option<VrLaunchMode>,
) -> Vec<LaunchRecipe> {
    try_launch_steam_profile(
        sim_name,
        app_id,
        vr_enabled,
        preferred_vr_launch_mode,
        None,
        true,
    )
    .expect("the curated Steam profile should launch")
}

fn try_launch_steam_profile(
    sim_name: &str,
    app_id: u32,
    vr_enabled: bool,
    preferred_vr_launch_mode: Option<VrLaunchMode>,
    override_recipe: Option<LaunchRecipe>,
    start_session: bool,
) -> Result<Vec<LaunchRecipe>, CoreError> {
    let storage = TempStorage::new();
    let launched_recipes = Arc::new(Mutex::new(Vec::new()));
    let runtime = RecipeRecordingRuntime {
        launched_recipes: Arc::clone(&launched_recipes),
        identity: ProcessIdentity {
            pid: 6_006,
            creation_time: "133822944900000000".to_owned(),
            canonical_executable_path: format!(r"C:\Games\{sim_name}\fixture.exe"),
        },
    };
    let mut core = FormationLapCore::open_with_runtime(storage.path(), runtime)
        .expect("core should open with the recipe recorder");
    let CommandOutcome::ProfileCreated { profile_id } = core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                sim_name.to_owned(),
                sim_name.to_owned(),
            )),
        })
        .expect("profile should be created")
    else {
        panic!("create should return the new profile id");
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("new profile should be selected");
    let primary_application_id = profile.primary_sim.id.clone();
    profile.primary_sim.launch_recipe = override_recipe.unwrap_or(LaunchRecipe {
        source: LaunchSource::Steam {
            app_id,
            selector: None,
        },
        arguments: Vec::new(),
        working_directory: None,
        monitored_process: None,
        monitored_executable_path: None,
        console_visibility: ConsoleVisibility::Hidden,
        elevated: false,
        startup_timeout_seconds: 30,
        post_start_delay_milliseconds: 0,
        shutdown_strategy: ShutdownStrategy::CloseWindows,
    });
    profile.primary_sim.path_needs_repair = false;
    profile.vr_enabled = vr_enabled;
    profile.preferred_vr_launch_mode = preferred_vr_launch_mode;
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("profile should save");

    if start_session {
        core.execute(AppCommand::StartSession { profile_id })?;
    } else {
        core.execute(AppCommand::StartApplication {
            profile_id,
            application_id: primary_application_id,
        })?;
    }

    let recipes = launched_recipes
        .lock()
        .expect("recipe trace should not be poisoned")
        .clone();
    Ok(recipes)
}
