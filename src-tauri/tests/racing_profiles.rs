use formation_lap_lib::{
    AppCommand, ApplicationIcon, ApplicationRequirement, CloseSessionSettings, CommandOutcome,
    ConsoleVisibility, FormationLapCore, LaunchRecipe, LaunchSource, ProfileApplication,
    ProfileReviewStatus, ProfileSummary, RacingProfile, ShutdownStrategy, SupportingApplication,
    TargetedDiscoverySources, VrLaunchMode,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
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
            "formation-lap-profile-test-{}-{timestamp}-{unique}",
            process::id()
        ));

        fs::create_dir_all(&path).expect("temporary profile storage should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

#[test]
fn saved_steam_primary_sim_uses_the_local_steam_library_icon() {
    let storage = TempStorage::new();
    let steam_root = storage.path().join("Steam");
    let library_cache = steam_root.join("appcache").join("librarycache");
    let steamapps = steam_root.join("steamapps");
    fs::create_dir_all(&library_cache).expect("Steam library icon cache should be created");
    fs::create_dir_all(steamapps.join("common").join("Automobilista 2"))
        .expect("Automobilista 2 installation should be created");
    fs::write(
        steamapps.join("appmanifest_1066890.acf"),
        r#""AppState"
{
  "appid" "1066890"
  "installdir" "Automobilista 2"
}"#,
    )
    .expect("Automobilista 2 manifest should be written");
    fs::write(
        library_cache.join("1066890_icon.png"),
        [0x89_u8, 0x50, 0x4e, 0x47],
    )
    .expect("local Automobilista 2 Steam icon should be written");
    let mut core = FormationLapCore::open_with_discovery_sources(
        storage.path(),
        TargetedDiscoverySources {
            steam_roots: vec![steam_root],
            ..TargetedDiscoverySources::default()
        },
    )
    .expect("FormationLapCore should open with local Steam metadata");
    core.execute(AppCommand::CreateProfile {
        profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
            "AMS2 race".to_owned(),
            "Automobilista 2".to_owned(),
        )),
    })
    .expect("a Racing Profile should be created");
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("the created Racing Profile should be selected");
    let primary_sim_application_id = profile.primary_sim.id.clone();
    profile.primary_sim.launch_recipe.source = LaunchSource::Steam {
        app_id: 1_066_890,
        selector: None,
    };
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("the Steam Primary Sim should be saved");

    let icon = core
        .snapshot()
        .application_icons
        .expect("saved Primary Sim icons should be included")
        .into_iter()
        .find(|icon| icon.application_id == primary_sim_application_id)
        .expect("the saved Automobilista 2 Primary Sim should have an icon");
    assert_eq!(
        icon.icon,
        ApplicationIcon::LocalData {
            media_type: "image/png".to_owned(),
            data_base64: "iVBORw==".to_owned(),
        }
    );

    let mut unresolved_direct_profile = core
        .snapshot()
        .selected_profile
        .expect("the saved Racing Profile should stay selected");
    unresolved_direct_profile.primary_sim.launch_recipe.source = LaunchSource::DirectExecutable {
        executable_path: String::new(),
    };
    unresolved_direct_profile.primary_sim.path_needs_repair = true;
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(unresolved_direct_profile),
    })
    .expect("the unresolved direct Primary Sim should be saved");
    let fallback_icon = core
        .snapshot()
        .application_icons
        .expect("saved Primary Sim icons should be included")
        .into_iter()
        .find(|icon| icon.application_id == primary_sim_application_id)
        .expect("the unresolved Automobilista 2 Primary Sim should have an icon");
    assert_eq!(
        fallback_icon.icon,
        ApplicationIcon::LocalData {
            media_type: "image/png".to_owned(),
            data_base64: "iVBORw==".to_owned(),
        }
    );
}

impl Drop for TempStorage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn created_racing_profile_survives_core_restart() {
    let storage = TempStorage::new();
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");

    let outcome = core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Le Mans Ultimate".to_owned(),
                "Le Mans Ultimate".to_owned(),
            )),
        })
        .expect("a valid Racing Profile should be created");
    let profile_id = match outcome {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let primary_sim_application_id = core
        .snapshot()
        .selected_profile
        .expect("the created Racing Profile should be selected")
        .primary_sim
        .id;

    drop(core);

    let reopened =
        FormationLapCore::open(storage.path()).expect("persisted profile storage should reopen");

    assert_eq!(
        reopened.snapshot().profiles,
        vec![ProfileSummary {
            id: profile_id,
            name: "Le Mans Ultimate".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
            primary_sim_application_id: Some(primary_sim_application_id),
            review_status: ProfileReviewStatus::Approved,
        }]
    );
}

#[test]
fn blank_racing_profile_names_are_rejected_without_changing_storage() {
    for (name, primary_sim_name) in [("   ", "Le Mans Ultimate"), ("Le Mans Ultimate", "\t\r\n")] {
        let storage = TempStorage::new();
        let mut core =
            FormationLapCore::open(storage.path()).expect("empty profile storage should open");

        let result = core.execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                name.to_owned(),
                primary_sim_name.to_owned(),
            )),
        });

        assert!(
            result.is_err(),
            "blank Racing Profile and Primary Sim names should be rejected"
        );
        assert!(
            core.snapshot().profiles.is_empty(),
            "rejected input should not change the authoritative snapshot"
        );
        assert_eq!(
            fs::read_dir(storage.path().join("profiles"))
                .expect("profile directory should remain readable")
                .count(),
            0,
            "rejected input should not write a profile document"
        );
    }
}

#[test]
fn persisted_blank_racing_profile_names_are_rejected_on_open() {
    for document in [
        r#"{
  "schemaVersion": 1,
  "id": "9c760ef8-79d8-446d-9d9d-21df5fc28b28",
  "name": " ",
  "primarySim": { "name": "Le Mans Ultimate" }
}"#,
        r#"{
  "schemaVersion": 1,
  "id": "9c760ef8-79d8-446d-9d9d-21df5fc28b28",
  "name": "Le Mans Ultimate",
  "primarySim": { "name": "\t" }
}"#,
    ] {
        let storage = TempStorage::new();
        let profiles = storage.path().join("profiles");
        fs::create_dir_all(&profiles).expect("profile directory should be created");
        fs::write(profiles.join("invalid.json"), document)
            .expect("invalid persisted profile fixture should be written");

        let result = FormationLapCore::open(storage.path());

        assert!(
            result.is_err(),
            "persisted blank Racing Profile names should not enter core state"
        );
    }
}

#[test]
fn edited_racing_profile_keeps_its_identity_after_restart() {
    let storage = TempStorage::new();
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Endurance".to_owned(),
                "Le Mans Ultimate".to_owned(),
            )),
        })
        .expect("a valid Racing Profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };

    let outcome = core
        .execute(AppCommand::EditProfile {
            profile_id: profile_id.clone(),
            name: "Sunday endurance".to_owned(),
            primary_sim_name: "rFactor 2".to_owned(),
        })
        .expect("an existing Racing Profile should be editable");

    assert_eq!(
        outcome,
        CommandOutcome::ProfileUpdated {
            profile_id: profile_id.clone()
        }
    );
    let primary_sim_application_id = core
        .snapshot()
        .selected_profile
        .expect("the edited Racing Profile should be selected")
        .primary_sim
        .id;

    drop(core);
    let reopened =
        FormationLapCore::open(storage.path()).expect("edited profile storage should reopen");
    assert_eq!(
        reopened.snapshot().profiles,
        vec![ProfileSummary {
            id: profile_id,
            name: "Sunday endurance".to_owned(),
            primary_sim_name: "rFactor 2".to_owned(),
            primary_sim_application_id: Some(primary_sim_application_id),
            review_status: ProfileReviewStatus::Approved,
        }]
    );
}

#[test]
fn deleted_racing_profile_stays_deleted_and_keeps_a_backup() {
    let storage = TempStorage::new();
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Sunday endurance".to_owned(),
                "Le Mans Ultimate".to_owned(),
            )),
        })
        .expect("a valid Racing Profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };

    let outcome = core
        .execute(AppCommand::DeleteProfile {
            profile_id: profile_id.clone(),
        })
        .expect("an existing Racing Profile should be deleted");

    assert_eq!(
        outcome,
        CommandOutcome::ProfileDeleted {
            profile_id: profile_id.clone()
        }
    );
    assert!(core.snapshot().profiles.is_empty());
    assert!(
        storage
            .path()
            .join("backups")
            .join(format!("{profile_id}.json"))
            .is_file(),
        "deletion should retain a recoverable profile document"
    );

    drop(core);
    let reopened =
        FormationLapCore::open(storage.path()).expect("profile storage should reopen after delete");
    assert!(reopened.snapshot().profiles.is_empty());
}

#[test]
fn duplicated_racing_profile_gets_a_new_identity_that_survives_restart() {
    let storage = TempStorage::new();
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");
    let source_profile_id = match core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Endurance".to_owned(),
                "Le Mans Ultimate".to_owned(),
            )),
        })
        .expect("a valid Racing Profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };

    let duplicate_profile_id = match core
        .execute(AppCommand::DuplicateProfile {
            source_profile_id: source_profile_id.clone(),
            name: "Endurance copy".to_owned(),
        })
        .expect("an existing Racing Profile should be duplicated")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected duplicate creation, got {other:?}"),
    };

    assert_ne!(duplicate_profile_id, source_profile_id);
    let primary_sim_application_ids = core
        .snapshot()
        .profiles
        .into_iter()
        .map(|summary| (summary.id, summary.primary_sim_application_id))
        .collect::<std::collections::HashMap<_, _>>();

    drop(core);
    let reopened =
        FormationLapCore::open(storage.path()).expect("duplicated profile storage should reopen");
    assert_eq!(
        reopened.snapshot().profiles,
        vec![
            ProfileSummary {
                primary_sim_application_id: primary_sim_application_ids
                    .get(&source_profile_id)
                    .expect("source profile icon identity should be present")
                    .clone(),
                id: source_profile_id,
                name: "Endurance".to_owned(),
                primary_sim_name: "Le Mans Ultimate".to_owned(),
                review_status: ProfileReviewStatus::Approved,
            },
            ProfileSummary {
                primary_sim_application_id: primary_sim_application_ids
                    .get(&duplicate_profile_id)
                    .expect("duplicate profile icon identity should be present")
                    .clone(),
                id: duplicate_profile_id,
                name: "Endurance copy".to_owned(),
                primary_sim_name: "Le Mans Ultimate".to_owned(),
                review_status: ProfileReviewStatus::Approved,
            },
        ]
    );
    let snapshot = reopened.snapshot();
    let icon_application_ids = snapshot
        .application_icons
        .expect("every persisted Racing Profile should contribute its local icon")
        .into_iter()
        .map(|icon| icon.application_id)
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        icon_application_ids,
        primary_sim_application_ids
            .into_values()
            .flatten()
            .collect::<std::collections::HashSet<_>>()
    );
}

#[test]
fn complete_racing_profile_configuration_survives_restart() {
    let storage = TempStorage::new();
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Le Mans evening".to_owned(),
                "Le Mans Ultimate".to_owned(),
            )),
        })
        .expect("a valid Racing Profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let configured_profile = RacingProfile {
        id: profile_id.clone(),
        name: "Le Mans evening".to_owned(),
        primary_sim: ProfileApplication {
            id: "f5a04482-c611-4e27-bb51-f467c307d76e".to_owned(),
            name: "Le Mans Ultimate".to_owned(),
            launch_recipe: LaunchRecipe {
                source: LaunchSource::Steam {
                    app_id: 2399420,
                    selector: None,
                },
                arguments: vec!["-vr".to_owned()],
                working_directory: None,
                monitored_process: Some("LeMansUltimate.exe".to_owned()),
                monitored_executable_path: None,
                console_visibility: ConsoleVisibility::Hidden,
                elevated: false,
                startup_timeout_seconds: 45,
                post_start_delay_milliseconds: 1_500,
                shutdown_strategy: ShutdownStrategy::CloseWindows,
            },
            path_needs_repair: false,
        },
        supporting_applications: vec![
            SupportingApplication {
                application: ProfileApplication {
                    id: "58968768-8710-4365-9839-9fc8dd4efad4".to_owned(),
                    name: "SimHub".to_owned(),
                    launch_recipe: LaunchRecipe {
                        source: LaunchSource::DirectExecutable {
                            executable_path: r"C:\Program Files\SimHub\SimHubWPF.exe".to_owned(),
                        },
                        arguments: vec!["-silent".to_owned()],
                        working_directory: Some(r"C:\Program Files\SimHub".to_owned()),
                        monitored_process: None,
                        monitored_executable_path: None,
                        console_visibility: ConsoleVisibility::Hidden,
                        elevated: false,
                        startup_timeout_seconds: 30,
                        post_start_delay_milliseconds: 500,
                        shutdown_strategy: ShutdownStrategy::CloseWindows,
                    },
                    path_needs_repair: true,
                },
                requirement: ApplicationRequirement::Required,
                keep_running: false,
            },
            SupportingApplication {
                application: ProfileApplication {
                    id: "209c8528-af6b-4c11-a186-de164032001f".to_owned(),
                    name: "Garage 61".to_owned(),
                    launch_recipe: LaunchRecipe {
                        source: LaunchSource::DirectExecutable {
                            executable_path: r"C:\Garage61\Garage61.Agent.exe".to_owned(),
                        },
                        arguments: Vec::new(),
                        working_directory: None,
                        monitored_process: None,
                        monitored_executable_path: None,
                        console_visibility: ConsoleVisibility::Visible,
                        elevated: true,
                        startup_timeout_seconds: 30,
                        post_start_delay_milliseconds: 0,
                        shutdown_strategy: ShutdownStrategy::ConsoleInterrupt,
                    },
                    path_needs_repair: true,
                },
                requirement: ApplicationRequirement::Optional,
                keep_running: true,
            },
        ],
        vr_enabled: true,
        preferred_vr_launch_mode: Some(VrLaunchMode::OpenXr),
        close_session: CloseSessionSettings {
            stop_steam_vr: true,
        },
    };

    let outcome = core
        .execute(AppCommand::SaveProfile {
            profile: Box::new(configured_profile.clone()),
        })
        .expect("a complete Racing Profile should be saved");
    assert_eq!(
        outcome,
        CommandOutcome::ProfileUpdated {
            profile_id: profile_id.clone()
        }
    );
    let authoritative_profile = core
        .snapshot()
        .selected_profile
        .expect("saved profile should remain selected");

    drop(core);
    let reopened =
        FormationLapCore::open(storage.path()).expect("configured profile storage should reopen");
    assert_eq!(
        reopened.snapshot().selected_profile,
        Some(authoritative_profile)
    );
}

#[test]
fn save_profile_retains_native_identity_and_recomputes_path_diagnostics() {
    let storage = TempStorage::new();
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Endurance".to_owned(),
                "Le Mans Ultimate".to_owned(),
            )),
        })
        .expect("a valid Racing Profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let original_primary_id = core
        .snapshot()
        .selected_profile
        .expect("created profile should be selected")
        .primary_sim
        .id;

    core.execute(AppCommand::SaveProfile {
        profile: Box::new(RacingProfile {
            id: profile_id,
            name: "Endurance".to_owned(),
            primary_sim: ProfileApplication {
                id: "frontend-primary".to_owned(),
                name: "Le Mans Ultimate".to_owned(),
                launch_recipe: LaunchRecipe {
                    source: LaunchSource::DirectExecutable {
                        executable_path: "Z:\\missing\\LeMansUltimate.exe".to_owned(),
                    },
                    ..LaunchRecipe::default()
                },
                path_needs_repair: false,
            },
            supporting_applications: vec![SupportingApplication {
                application: ProfileApplication {
                    id: "frontend-supporting".to_owned(),
                    name: "SimHub".to_owned(),
                    launch_recipe: LaunchRecipe {
                        source: LaunchSource::DirectExecutable {
                            executable_path: "Z:\\missing\\SimHub.exe".to_owned(),
                        },
                        ..LaunchRecipe::default()
                    },
                    path_needs_repair: false,
                },
                requirement: ApplicationRequirement::Optional,
                keep_running: true,
            }],
            vr_enabled: false,
            preferred_vr_launch_mode: None,
            close_session: CloseSessionSettings::default(),
        }),
    })
    .expect("valid frontend intent should be normalized and saved");

    let saved = core
        .snapshot()
        .selected_profile
        .expect("saved profile should remain selected");
    assert_eq!(saved.primary_sim.id, original_primary_id);
    assert!(saved.primary_sim.path_needs_repair);
    assert_ne!(
        saved.supporting_applications[0].application.id,
        "frontend-supporting"
    );
    assert!(
        saved.supporting_applications[0]
            .application
            .path_needs_repair
    );
}

#[test]
fn selected_racing_profile_survives_core_restart() {
    let storage = TempStorage::new();
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");
    let first_profile_id = match core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Assetto Corsa".to_owned(),
                "Assetto Corsa".to_owned(),
            )),
        })
        .expect("the first Racing Profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let selected_profile_id = match core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Le Mans Ultimate".to_owned(),
                "Le Mans Ultimate".to_owned(),
            )),
        })
        .expect("the second Racing Profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    assert_ne!(first_profile_id, selected_profile_id);

    let outcome = core
        .execute(AppCommand::SelectProfile {
            profile_id: selected_profile_id.clone(),
        })
        .expect("an existing Racing Profile should be selectable");
    assert_eq!(
        outcome,
        CommandOutcome::ProfileSelected {
            profile_id: selected_profile_id.clone()
        }
    );

    drop(core);
    let reopened = FormationLapCore::open(storage.path()).expect("profile selection should reopen");
    assert_eq!(
        reopened
            .snapshot()
            .selected_profile
            .expect("a selected profile should remain")
            .id,
        selected_profile_id
    );
}

#[test]
fn interrupted_settings_replacement_recovers_the_last_profile_selection() {
    let storage = TempStorage::new();
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");
    core.execute(AppCommand::CreateProfile {
        profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
            "First".to_owned(),
            "Assetto Corsa".to_owned(),
        )),
    })
    .expect("the first Racing Profile should be created");
    let selected_profile_id = match core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Selected".to_owned(),
                "Le Mans Ultimate".to_owned(),
            )),
        })
        .expect("the selected Racing Profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    core.execute(AppCommand::SelectProfile {
        profile_id: selected_profile_id.clone(),
    })
    .expect("the second Racing Profile should be selected");
    drop(core);

    let settings = storage.path().join("settings.json");
    let backup = storage.path().join("backups").join("settings.json");
    fs::rename(&settings, &backup)
        .expect("fixture should simulate moving the last valid settings to backup");
    fs::write(storage.path().join("settings.json.tmp"), b"{ interrupted")
        .expect("fixture should leave an incomplete settings replacement");

    let recovered =
        FormationLapCore::open(storage.path()).expect("the last valid settings should recover");

    assert_eq!(
        recovered
            .snapshot()
            .selected_profile
            .expect("the selected Racing Profile should recover")
            .id,
        selected_profile_id
    );
}

#[test]
fn interrupted_profile_replacement_recovers_the_last_valid_document() {
    let storage = TempStorage::new();
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Le Mans Ultimate".to_owned(),
                "Le Mans Ultimate".to_owned(),
            )),
        })
        .expect("a valid Racing Profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let primary_sim_application_id = core
        .snapshot()
        .selected_profile
        .expect("the created Racing Profile should be selected")
        .primary_sim
        .id;
    drop(core);

    let live_document = storage
        .path()
        .join("profiles")
        .join(format!("{profile_id}.json"));
    let backup_document = storage
        .path()
        .join("backups")
        .join(format!("{profile_id}.json"));
    fs::rename(&live_document, &backup_document)
        .expect("fixture should simulate moving the last valid document to backup");
    fs::write(
        storage
            .path()
            .join("profiles")
            .join(format!(".{profile_id}.json.tmp")),
        b"{ interrupted",
    )
    .expect("fixture should leave an incomplete replacement");

    let recovered =
        FormationLapCore::open(storage.path()).expect("the last valid profile should recover");

    assert_eq!(
        recovered.snapshot().profiles,
        vec![ProfileSummary {
            id: profile_id,
            name: "Le Mans Ultimate".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
            primary_sim_application_id: Some(primary_sim_application_id),
            review_status: ProfileReviewStatus::Approved,
        }]
    );
}

#[test]
fn invalid_profile_replacement_recovers_the_last_valid_document() {
    let storage = TempStorage::new();
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Last valid".to_owned(),
                "Le Mans Ultimate".to_owned(),
            )),
        })
        .expect("a valid Racing Profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let mut edited = core
        .snapshot()
        .selected_profile
        .expect("the created Racing Profile should be selected");
    edited.name = "Invalid replacement".to_owned();
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(edited),
    })
    .expect("editing should retain the last valid document as a backup");
    let primary_sim_application_id = core
        .snapshot()
        .selected_profile
        .expect("the saved Racing Profile should be selected")
        .primary_sim
        .id;
    drop(core);

    let live_document = storage
        .path()
        .join("profiles")
        .join(format!("{profile_id}.json"));
    fs::write(&live_document, b"{ invalid replacement")
        .expect("fixture should corrupt only the live replacement");

    let recovered =
        FormationLapCore::open(storage.path()).expect("the last valid profile should recover");

    assert_eq!(
        recovered.snapshot().profiles,
        vec![ProfileSummary {
            id: profile_id,
            name: "Last valid".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
            primary_sim_application_id: Some(primary_sim_application_id),
            review_status: ProfileReviewStatus::Approved,
        }]
    );
}

#[test]
fn schema_one_profile_is_migrated_without_losing_identity() {
    let storage = TempStorage::new();
    let profiles = storage.path().join("profiles");
    fs::create_dir_all(&profiles).expect("profile directory should be created");
    let profile_id = "1ea99c98-51b0-4cb8-89ea-04440c26e6d7";
    let profile_path = profiles.join(format!("{profile_id}.json"));
    fs::write(
        &profile_path,
        format!(
            r#"{{
  "schemaVersion": 1,
  "id": "{profile_id}",
  "name": "Le Mans evening",
  "primarySim": {{
    "name": "Le Mans Ultimate"
  }}
}}"#
        ),
    )
    .expect("schema-one profile fixture should be written");

    let core =
        FormationLapCore::open(storage.path()).expect("schema-one profile should be migrated");
    let migrated = core
        .snapshot()
        .selected_profile
        .expect("migrated profile should remain selected");
    assert_eq!(migrated.id, profile_id);
    assert_eq!(migrated.name, "Le Mans evening");
    assert_eq!(migrated.primary_sim.name, "Le Mans Ultimate");
    assert!(migrated.primary_sim.path_needs_repair);

    let persisted: serde_json::Value = serde_json::from_slice(
        &fs::read(&profile_path).expect("migrated profile document should remain readable"),
    )
    .expect("migrated profile document should remain valid JSON");
    assert_eq!(persisted["schemaVersion"], 2);
    assert!(
        storage
            .path()
            .join("backups")
            .join(format!("{profile_id}.json"))
            .is_file(),
        "migration should retain the prior schema-one document"
    );
}

#[test]
fn invalid_legacy_profile_identity_is_repaired_without_selecting_a_filesystem_path() {
    let storage = TempStorage::new();
    let profiles = storage.path().join("profiles");
    fs::create_dir_all(&profiles).expect("profile directory should be created");
    let legacy_path = profiles.join("portable-profile.json");
    fs::write(
        &legacy_path,
        r#"{
  "schemaVersion": 2,
  "id": "../outside-profile",
  "name": "Imported legacy profile",
  "primarySim": {
    "name": "Le Mans Ultimate"
  }
}"#,
    )
    .expect("invalid legacy profile fixture should be written");
    let outside_path = storage.path().join("outside-profile.json");
    fs::write(&outside_path, b"must remain untouched").expect("outside sentinel should be written");

    let mut core = FormationLapCore::open(storage.path())
        .expect("invalid legacy identity should be repaired during open");
    let repaired = core
        .snapshot()
        .selected_profile
        .expect("repaired profile should remain available");
    assert_eq!(
        uuid::Uuid::parse_str(&repaired.id)
            .expect("repaired profile ID should be a UUID")
            .to_string(),
        repaired.id
    );
    assert_ne!(repaired.id, "../outside-profile");
    assert!(!legacy_path.exists());
    assert!(
        profiles.join(format!("{}.json", repaired.id)).is_file(),
        "repaired profile should use its UUID-backed filename"
    );
    let legacy_backup = storage
        .path()
        .join("backups")
        .join(format!("{}.legacy.json", repaired.id));
    assert!(
        legacy_backup.is_file(),
        "the invalid source document should remain recoverable"
    );
    let backup: serde_json::Value = serde_json::from_slice(
        &fs::read(legacy_backup).expect("legacy backup should remain readable"),
    )
    .expect("legacy backup should remain valid JSON");
    assert_eq!(backup["id"], "../outside-profile");

    let repaired_id = repaired.id.clone();
    let mut edited = repaired;
    edited.name = "Repaired and saved".to_owned();
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(edited),
    })
    .expect("repaired profile should save through its trusted source path");
    core.execute(AppCommand::DeleteProfile {
        profile_id: repaired_id.clone(),
    })
    .expect("repaired profile should delete through its trusted source path");
    assert!(!profiles.join(format!("{repaired_id}.json")).exists());
    assert!(
        storage
            .path()
            .join("backups")
            .join(format!("{repaired_id}.json"))
            .is_file(),
        "deletion should retain the last repaired profile document"
    );
    assert_eq!(
        fs::read(&outside_path).expect("outside sentinel should remain readable"),
        b"must remain untouched"
    );
}

#[test]
fn exported_racing_profile_is_portable_and_contains_no_runtime_identity() {
    let storage = TempStorage::new();
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Le Mans evening".to_owned(),
                "Le Mans Ultimate".to_owned(),
            )),
        })
        .expect("a valid Racing Profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };

    let document = match core
        .execute(AppCommand::ExportProfile { profile_id })
        .expect("an existing Racing Profile should export")
    {
        CommandOutcome::ProfileExported { document } => document,
        other => panic!("expected profile export, got {other:?}"),
    };

    assert!(!document.contains("\"id\""));
    assert!(!document.contains("pathNeedsRepair"));
    assert!(!document.contains("processIdentity"));
    let actual: serde_json::Value =
        serde_json::from_str(&document).expect("export should be valid JSON");
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/exported-profile.json"))
            .expect("example exported profile fixture should be valid JSON");
    assert_eq!(actual, expected);
}

#[test]
fn imported_profile_gets_fresh_identity_and_marks_missing_paths_for_repair() {
    let storage = TempStorage::new();
    let mut portable: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/exported-profile.json"))
            .expect("example exported profile fixture should be valid JSON");
    portable["id"] = serde_json::json!("transient-profile-id");
    portable["primarySim"]["id"] = serde_json::json!("transient-entry-id");
    portable["primarySim"]["processIdentity"] = serde_json::json!({
        "pid": 42,
        "creationTime": 1234
    });
    portable["primarySim"]["launchRecipe"]["source"]["executablePath"] =
        serde_json::json!(r"C:\Missing\LeMansUltimate.exe");
    let document =
        serde_json::to_string_pretty(&portable).expect("portable fixture should serialize");
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");

    let profile_id = match core
        .execute(AppCommand::ImportProfile { document })
        .expect("a portable Racing Profile should import")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected imported profile creation, got {other:?}"),
    };

    let imported = core
        .snapshot()
        .selected_profile
        .expect("the imported Racing Profile should be selected");
    assert_eq!(imported.id, profile_id);
    assert_ne!(imported.id, "transient-profile-id");
    assert_ne!(imported.primary_sim.id, "transient-entry-id");
    assert_eq!(
        imported.primary_sim.launch_recipe.source,
        LaunchSource::DirectExecutable {
            executable_path: r"C:\Missing\LeMansUltimate.exe".to_owned()
        }
    );
    assert!(imported.primary_sim.path_needs_repair);
    assert!(
        !serde_json::to_string(&imported)
            .expect("imported profile should serialize")
            .contains("processIdentity")
    );
}

#[test]
fn newly_imported_profile_cannot_start_until_its_configuration_is_approved() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let mut portable: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/exported-profile.json"))
            .expect("example exported profile fixture should be valid JSON");
    portable["primarySim"]["launchRecipe"]["source"]["executablePath"] =
        serde_json::json!(executable_path);
    portable["primarySim"]["launchRecipe"]["workingDirectory"] =
        serde_json::json!(executable_path.parent());
    let document =
        serde_json::to_string_pretty(&portable).expect("portable fixture should serialize");
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");

    let profile_id = match core
        .execute(AppCommand::ImportProfile { document })
        .expect("a portable Racing Profile should import")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected imported profile creation, got {other:?}"),
    };

    assert_eq!(
        core.snapshot().profiles[0].review_status,
        ProfileReviewStatus::NeedsReview
    );
    assert!(matches!(
        core.execute(AppCommand::StartSession {
            profile_id: profile_id.clone(),
        }),
        Err(formation_lap_lib::CoreError::ProfileNeedsReview(id)) if id == profile_id
    ));

    core.execute(AppCommand::ApproveProfile {
        profile_id,
        configuration_reviewed: true,
        approved_privileged_application_ids: Vec::new(),
    })
    .expect("reviewed non-privileged configuration should be approved");

    assert_eq!(
        core.snapshot().profiles[0].review_status,
        ProfileReviewStatus::Approved
    );
}

#[test]
fn imported_elevated_and_custom_stop_entries_require_explicit_approval() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let mut portable: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/exported-profile.json"))
            .expect("example exported profile fixture should be valid JSON");
    portable["primarySim"]["launchRecipe"]["source"]["executablePath"] =
        serde_json::json!(executable_path);
    portable["primarySim"]["launchRecipe"]["elevated"] = serde_json::json!(true);
    portable["supportingApplications"] = serde_json::json!([{
        "application": {
            "name": "Reviewed custom stop",
            "launchRecipe": {
                "source": {
                    "kind": "directExecutable",
                    "executablePath": executable_path
                },
                "arguments": [],
                "workingDirectory": executable_path.parent(),
                "monitoredProcess": null,
                "monitoredExecutablePath": null,
                "consoleVisibility": "hidden",
                "elevated": false,
                "startupTimeoutSeconds": 30,
                "postStartDelayMilliseconds": 0,
                "shutdownStrategy": {
                    "kind": "customStop",
                    "executablePath": executable_path,
                    "arguments": ["--stop"]
                }
            }
        },
        "requirement": "optional",
        "keepRunning": false
    }]);
    let document =
        serde_json::to_string_pretty(&portable).expect("portable fixture should serialize");
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");
    let profile_id = match core
        .execute(AppCommand::ImportProfile { document })
        .expect("a portable Racing Profile should import")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected imported profile creation, got {other:?}"),
    };
    let profile = core
        .snapshot()
        .selected_profile
        .expect("the imported Racing Profile should be selected");

    assert!(matches!(
        core.execute(AppCommand::ApproveProfile {
            profile_id: profile_id.clone(),
            configuration_reviewed: true,
            approved_privileged_application_ids: vec![profile.primary_sim.id.clone()],
        }),
        Err(formation_lap_lib::CoreError::InvalidProfileApproval(_))
    ));

    core.execute(AppCommand::ApproveProfile {
        profile_id,
        configuration_reviewed: true,
        approved_privileged_application_ids: vec![
            profile.primary_sim.id,
            profile.supporting_applications[0].application.id.clone(),
        ],
    })
    .expect("every privileged entry was approved");
    assert_eq!(
        core.snapshot().profiles[0].review_status,
        ProfileReviewStatus::Approved
    );
}

#[test]
fn imported_profile_quarantines_missing_secondary_executable_paths() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let mut portable: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/exported-profile.json"))
            .expect("example exported profile fixture should be valid JSON");
    portable["primarySim"]["launchRecipe"]["source"]["executablePath"] =
        serde_json::json!(executable_path);
    portable["primarySim"]["launchRecipe"]["workingDirectory"] =
        serde_json::json!(r"C:\Missing\Working");
    portable["primarySim"]["launchRecipe"]["monitoredExecutablePath"] =
        serde_json::json!(r"C:\Missing\Observed.exe");
    portable["primarySim"]["launchRecipe"]["shutdownStrategy"] = serde_json::json!({
        "kind": "customStop",
        "executablePath": r"C:\Missing\Stop.exe",
        "arguments": []
    });
    let document =
        serde_json::to_string_pretty(&portable).expect("portable fixture should serialize");
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");
    let profile_id = match core
        .execute(AppCommand::ImportProfile { document })
        .expect("a portable Racing Profile should import")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected imported profile creation, got {other:?}"),
    };
    let profile = core
        .snapshot()
        .selected_profile
        .expect("the imported Racing Profile should be selected");

    assert!(
        profile.primary_sim.path_needs_repair,
        "working, monitored, and custom-stop executable paths participate in review"
    );
    assert!(matches!(
        core.execute(AppCommand::ApproveProfile {
            profile_id,
            configuration_reviewed: true,
            approved_privileged_application_ids: vec![profile.primary_sim.id],
        }),
        Err(formation_lap_lib::CoreError::InvalidProfileApproval(_))
    ));
}

#[test]
fn editing_an_approved_privileged_recipe_invalidates_its_approval() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Privileged local profile".to_owned(),
                "Fixture".to_owned(),
            )),
        })
        .expect("a valid Racing Profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let mut profile = core
        .snapshot()
        .selected_profile
        .expect("the created Racing Profile should be selected");
    profile.primary_sim.launch_recipe.source = LaunchSource::DirectExecutable {
        executable_path: executable_path.to_string_lossy().into_owned(),
    };
    profile.primary_sim.launch_recipe.elevated = true;
    profile.primary_sim.launch_recipe.shutdown_strategy = ShutdownStrategy::CustomStop {
        executable_path: executable_path.to_string_lossy().into_owned(),
        arguments: vec!["--stop".to_owned()],
    };
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("the privileged recipe should save into review quarantine");
    let primary_id = core
        .snapshot()
        .selected_profile
        .expect("the profile should remain selected")
        .primary_sim
        .id;
    core.execute(AppCommand::ApproveProfile {
        profile_id: profile_id.clone(),
        configuration_reviewed: true,
        approved_privileged_application_ids: vec![primary_id.clone()],
    })
    .expect("the privileged recipe should be approved");

    let mut changed_arguments = core
        .snapshot()
        .selected_profile
        .expect("the profile should remain selected");
    changed_arguments.primary_sim.launch_recipe.arguments = vec!["--changed".to_owned()];
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(changed_arguments),
    })
    .expect("changed elevated arguments should save into review quarantine");
    assert_eq!(
        core.snapshot().profiles[0].review_status,
        ProfileReviewStatus::NeedsReview
    );

    core.execute(AppCommand::ApproveProfile {
        profile_id,
        configuration_reviewed: true,
        approved_privileged_application_ids: vec![primary_id],
    })
    .expect("the changed elevated recipe should be approved");
    let mut changed_custom_stop = core
        .snapshot()
        .selected_profile
        .expect("the profile should remain selected");
    changed_custom_stop
        .primary_sim
        .launch_recipe
        .shutdown_strategy = ShutdownStrategy::CustomStop {
        executable_path: executable_path.to_string_lossy().into_owned(),
        arguments: vec!["--different-stop".to_owned()],
    };
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(changed_custom_stop),
    })
    .expect("changed custom-stop arguments should save into review quarantine");
    assert_eq!(
        core.snapshot().profiles[0].review_status,
        ProfileReviewStatus::NeedsReview
    );
}

#[test]
fn an_approved_privileged_recipe_remains_approved_after_restart_when_unchanged() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let profile_id;
    {
        let mut core =
            FormationLapCore::open(storage.path()).expect("empty profile storage should open");
        profile_id = match core
            .execute(AppCommand::CreateProfile {
                profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                    "Approved local profile".to_owned(),
                    "Fixture".to_owned(),
                )),
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
        profile.primary_sim.launch_recipe.source = LaunchSource::DirectExecutable {
            executable_path: executable_path.to_string_lossy().into_owned(),
        };
        profile.primary_sim.launch_recipe.elevated = true;
        core.execute(AppCommand::SaveProfile {
            profile: Box::new(profile),
        })
        .expect("the privileged recipe should enter review quarantine");
        let primary_id = core
            .snapshot()
            .selected_profile
            .expect("fixture profile should remain selected")
            .primary_sim
            .id;
        core.execute(AppCommand::ApproveProfile {
            profile_id: profile_id.clone(),
            configuration_reviewed: true,
            approved_privileged_application_ids: vec![primary_id],
        })
        .expect("the privileged recipe should be approved");
    }

    let reopened =
        FormationLapCore::open(storage.path()).expect("approved local storage should reopen");
    assert_eq!(
        reopened.snapshot().profiles[0].review_status,
        ProfileReviewStatus::Approved
    );
}

#[test]
fn a_missing_protected_approval_requarantines_an_approved_privileged_recipe() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let profile_id;
    {
        let mut core =
            FormationLapCore::open(storage.path()).expect("empty profile storage should open");
        profile_id = match core
            .execute(AppCommand::CreateProfile {
                profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                    "Approval record fixture".to_owned(),
                    "Fixture".to_owned(),
                )),
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
        profile.primary_sim.launch_recipe.source = LaunchSource::DirectExecutable {
            executable_path: executable_path.to_string_lossy().into_owned(),
        };
        profile.primary_sim.launch_recipe.elevated = true;
        core.execute(AppCommand::SaveProfile {
            profile: Box::new(profile),
        })
        .expect("the privileged recipe should enter review quarantine");
        let primary_id = core
            .snapshot()
            .selected_profile
            .expect("fixture profile should remain selected")
            .primary_sim
            .id;
        core.execute(AppCommand::ApproveProfile {
            profile_id: profile_id.clone(),
            configuration_reviewed: true,
            approved_privileged_application_ids: vec![primary_id],
        })
        .expect("the privileged recipe should be approved");
    }

    fs::remove_file(
        storage
            .path()
            .join("profile-approvals")
            .join(format!("{profile_id}.bin")),
    )
    .expect("approval record should be removable for the fixture");

    let reopened = FormationLapCore::open(storage.path())
        .expect("local storage should reopen without a record");
    assert_eq!(
        reopened.snapshot().profiles[0].review_status,
        ProfileReviewStatus::NeedsReview
    );
}

#[test]
fn duplicating_an_approved_privileged_recipe_does_not_transfer_its_approval() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");
    let source_profile_id = match core
        .execute(AppCommand::CreateProfile {
            profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                "Approved source".to_owned(),
                "Fixture".to_owned(),
            )),
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
    profile.primary_sim.launch_recipe.source = LaunchSource::DirectExecutable {
        executable_path: executable_path.to_string_lossy().into_owned(),
    };
    profile.primary_sim.launch_recipe.elevated = true;
    core.execute(AppCommand::SaveProfile {
        profile: Box::new(profile),
    })
    .expect("the privileged recipe should enter review quarantine");
    let primary_id = core
        .snapshot()
        .selected_profile
        .expect("fixture profile should remain selected")
        .primary_sim
        .id;
    core.execute(AppCommand::ApproveProfile {
        profile_id: source_profile_id.clone(),
        configuration_reviewed: true,
        approved_privileged_application_ids: vec![primary_id],
    })
    .expect("the source recipe should be approved");

    let duplicate_profile_id = match core
        .execute(AppCommand::DuplicateProfile {
            source_profile_id: source_profile_id.clone(),
            name: "Unreviewed copy".to_owned(),
        })
        .expect("approved source should be duplicable")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected duplicate creation, got {other:?}"),
    };

    let review_statuses = core
        .snapshot()
        .profiles
        .into_iter()
        .map(|profile| (profile.id, profile.review_status))
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(
        review_statuses.get(&source_profile_id),
        Some(&ProfileReviewStatus::Approved)
    );
    assert_eq!(
        review_statuses.get(&duplicate_profile_id),
        Some(&ProfileReviewStatus::NeedsReview)
    );

    drop(core);
    let reopened = FormationLapCore::open(storage.path()).expect("profile storage should reopen");
    let reopened_statuses = reopened
        .snapshot()
        .profiles
        .into_iter()
        .map(|profile| (profile.id, profile.review_status))
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(
        reopened_statuses.get(&source_profile_id),
        Some(&ProfileReviewStatus::Approved)
    );
    assert_eq!(
        reopened_statuses.get(&duplicate_profile_id),
        Some(&ProfileReviewStatus::NeedsReview)
    );
}

#[test]
fn a_disk_modified_privileged_recipe_cannot_reuse_a_persisted_approval_after_restart() {
    let storage = TempStorage::new();
    let executable_path = std::env::current_exe()
        .expect("test executable path should be available")
        .canonicalize()
        .expect("test executable path should canonicalize");
    let profile_id;
    {
        let mut core =
            FormationLapCore::open(storage.path()).expect("empty profile storage should open");
        profile_id = match core
            .execute(AppCommand::CreateProfile {
                profile: Box::new(formation_lap_lib::NewRacingProfile::from_names(
                    "Privileged local profile".to_owned(),
                    "Fixture".to_owned(),
                )),
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
        profile.primary_sim.launch_recipe.source = LaunchSource::DirectExecutable {
            executable_path: executable_path.to_string_lossy().into_owned(),
        };
        profile.primary_sim.launch_recipe.elevated = true;
        core.execute(AppCommand::SaveProfile {
            profile: Box::new(profile),
        })
        .expect("the privileged recipe should enter review quarantine");
        let primary_id = core
            .snapshot()
            .selected_profile
            .expect("fixture profile should remain selected")
            .primary_sim
            .id;
        core.execute(AppCommand::ApproveProfile {
            profile_id: profile_id.clone(),
            configuration_reviewed: true,
            approved_privileged_application_ids: vec![primary_id],
        })
        .expect("the privileged recipe should be approved");
    }

    let profile_path = storage
        .path()
        .join("profiles")
        .join(format!("{profile_id}.json"));
    let mut document: serde_json::Value = serde_json::from_slice(
        &fs::read(&profile_path).expect("profile document should be readable"),
    )
    .expect("profile document should parse");
    document["primarySim"]["launchRecipe"]["arguments"] = serde_json::json!(["--tampered"]);
    fs::write(
        &profile_path,
        serde_json::to_vec_pretty(&document).expect("tampered profile should serialize"),
    )
    .expect("tampered profile should be written");

    let mut reopened =
        FormationLapCore::open(storage.path()).expect("tampered local storage should reopen");
    assert_eq!(
        reopened.snapshot().profiles[0].review_status,
        ProfileReviewStatus::NeedsReview
    );
    assert!(matches!(
        reopened.execute(AppCommand::StartSession { profile_id }),
        Err(formation_lap_lib::CoreError::ProfileNeedsReview(_))
    ));
}
