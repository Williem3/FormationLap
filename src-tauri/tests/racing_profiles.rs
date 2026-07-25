use formation_lap_lib::{
    AppCommand, ApplicationRequirement, CloseSessionSettings, CommandOutcome, ConsoleVisibility,
    FormationLapCore, LaunchRecipe, LaunchSource, ProfileApplication, ProfileSummary,
    RacingProfile, ShutdownStrategy, SupportingApplication, VrLaunchMode,
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
            name: "Le Mans Ultimate".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
        })
        .expect("a valid Racing Profile should be created");
    let profile_id = match outcome {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };

    drop(core);

    let reopened =
        FormationLapCore::open(storage.path()).expect("persisted profile storage should reopen");

    assert_eq!(
        reopened.snapshot().profiles,
        vec![ProfileSummary {
            id: profile_id,
            name: "Le Mans Ultimate".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
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
            name: name.to_owned(),
            primary_sim_name: primary_sim_name.to_owned(),
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
            name: "Endurance".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
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

    drop(core);
    let reopened =
        FormationLapCore::open(storage.path()).expect("edited profile storage should reopen");
    assert_eq!(
        reopened.snapshot().profiles,
        vec![ProfileSummary {
            id: profile_id,
            name: "Sunday endurance".to_owned(),
            primary_sim_name: "rFactor 2".to_owned(),
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
            name: "Sunday endurance".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
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
            name: "Endurance".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
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

    drop(core);
    let reopened =
        FormationLapCore::open(storage.path()).expect("duplicated profile storage should reopen");
    assert_eq!(
        reopened.snapshot().profiles,
        vec![
            ProfileSummary {
                id: source_profile_id,
                name: "Endurance".to_owned(),
                primary_sim_name: "Le Mans Ultimate".to_owned(),
            },
            ProfileSummary {
                id: duplicate_profile_id,
                name: "Endurance copy".to_owned(),
                primary_sim_name: "Le Mans Ultimate".to_owned(),
            },
        ]
    );
}

#[test]
fn complete_racing_profile_configuration_survives_restart() {
    let storage = TempStorage::new();
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Le Mans evening".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
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
            name: "Endurance".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
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
            name: "Assetto Corsa".to_owned(),
            primary_sim_name: "Assetto Corsa".to_owned(),
        })
        .expect("the first Racing Profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
    let selected_profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Le Mans Ultimate".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
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
        name: "First".to_owned(),
        primary_sim_name: "Assetto Corsa".to_owned(),
    })
    .expect("the first Racing Profile should be created");
    let selected_profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Selected".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
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
            name: "Le Mans Ultimate".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
        })
        .expect("a valid Racing Profile should be created")
    {
        CommandOutcome::ProfileCreated { profile_id } => profile_id,
        other => panic!("expected profile creation, got {other:?}"),
    };
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
            name: "Last valid".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
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
fn exported_racing_profile_is_portable_and_contains_no_runtime_identity() {
    let storage = TempStorage::new();
    let mut core =
        FormationLapCore::open(storage.path()).expect("empty profile storage should open");
    let profile_id = match core
        .execute(AppCommand::CreateProfile {
            name: "Le Mans evening".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
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
