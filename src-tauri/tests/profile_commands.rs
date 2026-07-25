use formation_lap_lib::{
    ApproveProfilePayload, CreateProfilePayload, DuplicateProfilePayload, ImportProfilePayload,
    NativeCommandHost, ProfileIdPayload, ProfileReviewStatus, SaveProfilePayload,
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
            "formation-lap-command-test-{}-{timestamp}-{unique}",
            process::id()
        ));
        fs::create_dir_all(&path).expect("temporary command storage should be created");
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
fn create_profile_command_returns_the_authoritative_snapshot() {
    let storage = TempStorage::new();
    let commands =
        NativeCommandHost::open(storage.path()).expect("native command host should open");

    let snapshot = commands
        .create_profile(CreateProfilePayload {
            name: "Le Mans evening".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
        })
        .expect("valid command payload should create a Racing Profile");

    assert_eq!(snapshot.profiles.len(), 1);
    assert_eq!(snapshot.profiles[0].name, "Le Mans evening");
    assert_eq!(snapshot.profiles[0].primary_sim_name, "Le Mans Ultimate");
    assert_eq!(
        snapshot
            .selected_profile
            .expect("created profile should be selected")
            .id,
        snapshot.profiles[0].id
    );
}

#[test]
fn save_profile_command_returns_the_updated_authoritative_snapshot() {
    let storage = TempStorage::new();
    let commands =
        NativeCommandHost::open(storage.path()).expect("native command host should open");
    let mut profile = commands
        .create_profile(CreateProfilePayload {
            name: "Endurance".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
        })
        .expect("valid command payload should create a Racing Profile")
        .selected_profile
        .expect("created profile should be selected");
    profile.name = "Sunday endurance".to_owned();

    let snapshot = commands
        .save_profile(SaveProfilePayload { profile })
        .expect("valid command payload should save a Racing Profile");

    assert_eq!(snapshot.profiles[0].name, "Sunday endurance");
    assert_eq!(
        snapshot
            .selected_profile
            .expect("saved profile should remain selected")
            .name,
        "Sunday endurance"
    );
}

#[test]
fn select_profile_command_changes_the_authoritative_snapshot() {
    let storage = TempStorage::new();
    let commands =
        NativeCommandHost::open(storage.path()).expect("native command host should open");
    commands
        .create_profile(CreateProfilePayload {
            name: "Assetto Corsa".to_owned(),
            primary_sim_name: "Assetto Corsa".to_owned(),
        })
        .expect("the first Racing Profile should be created");
    let second_profile_id = commands
        .create_profile(CreateProfilePayload {
            name: "Le Mans Ultimate".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
        })
        .expect("the second Racing Profile should be created")
        .profiles
        .into_iter()
        .find(|profile| profile.name == "Le Mans Ultimate")
        .expect("the second Racing Profile should be listed")
        .id;

    let snapshot = commands
        .select_profile(ProfileIdPayload {
            profile_id: second_profile_id.clone(),
        })
        .expect("an existing Racing Profile should be selectable");

    assert_eq!(
        snapshot
            .selected_profile
            .expect("selection should be returned")
            .id,
        second_profile_id
    );
}

#[test]
fn duplicate_profile_command_returns_both_racing_profiles() {
    let storage = TempStorage::new();
    let commands =
        NativeCommandHost::open(storage.path()).expect("native command host should open");
    let source_profile_id = commands
        .create_profile(CreateProfilePayload {
            name: "Endurance".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
        })
        .expect("source Racing Profile should be created")
        .profiles[0]
        .id
        .clone();

    let snapshot = commands
        .duplicate_profile(DuplicateProfilePayload {
            source_profile_id: source_profile_id.clone(),
            name: "Endurance copy".to_owned(),
        })
        .expect("an existing Racing Profile should be duplicated");

    assert_eq!(snapshot.profiles.len(), 2);
    let duplicate = snapshot
        .profiles
        .iter()
        .find(|profile| profile.name == "Endurance copy")
        .expect("duplicate should be returned");
    assert_ne!(duplicate.id, source_profile_id);
    assert_eq!(duplicate.primary_sim_name, "Le Mans Ultimate");
}

#[test]
fn delete_profile_command_returns_the_remaining_authoritative_snapshot() {
    let storage = TempStorage::new();
    let commands =
        NativeCommandHost::open(storage.path()).expect("native command host should open");
    let profile_id = commands
        .create_profile(CreateProfilePayload {
            name: "Endurance".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
        })
        .expect("Racing Profile should be created")
        .profiles[0]
        .id
        .clone();

    let snapshot = commands
        .delete_profile(ProfileIdPayload { profile_id })
        .expect("an existing Racing Profile should be deleted");

    assert!(snapshot.profiles.is_empty());
    assert!(snapshot.selected_profile.is_none());
}

#[test]
fn export_and_import_commands_round_trip_a_portable_racing_profile() {
    let source_storage = TempStorage::new();
    let source_commands =
        NativeCommandHost::open(source_storage.path()).expect("source command host should open");
    let profile_id = source_commands
        .create_profile(CreateProfilePayload {
            name: "Le Mans evening".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
        })
        .expect("Racing Profile should be created")
        .profiles[0]
        .id
        .clone();
    let document = source_commands
        .export_profile(ProfileIdPayload { profile_id })
        .expect("Racing Profile should export");
    let target_storage = TempStorage::new();
    let target_commands =
        NativeCommandHost::open(target_storage.path()).expect("target command host should open");

    let snapshot = target_commands
        .import_profile(ImportProfilePayload { document })
        .expect("portable Racing Profile should import");

    assert_eq!(snapshot.profiles.len(), 1);
    assert_eq!(snapshot.profiles[0].name, "Le Mans evening");
    assert_eq!(snapshot.profiles[0].primary_sim_name, "Le Mans Ultimate");
}

#[test]
fn import_command_exposes_review_state_and_native_approval() {
    let storage = TempStorage::new();
    let commands =
        NativeCommandHost::open(storage.path()).expect("native command host should open");
    let mut portable: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/exported-profile.json"))
            .expect("portable fixture should be valid JSON");
    portable["primarySim"]["launchRecipe"]["source"] = serde_json::json!({
        "kind": "steam",
        "appId": 1623730,
        "selector": null
    });
    let document =
        serde_json::to_string_pretty(&portable).expect("portable fixture should remain valid JSON");

    let imported = commands
        .import_profile(ImportProfilePayload { document })
        .expect("portable Racing Profile should import");
    let profile_id = imported.profiles[0].id.clone();
    assert_eq!(
        imported.profiles[0].review_status,
        ProfileReviewStatus::NeedsReview
    );
    assert_eq!(
        commands
            .start_session(ProfileIdPayload {
                profile_id: profile_id.clone(),
            })
            .expect_err("Session start should remain quarantined")
            .code,
        "profile_needs_review"
    );

    let approved = commands
        .approve_profile(ApproveProfilePayload {
            profile_id,
            configuration_reviewed: true,
            approved_privileged_application_ids: Vec::new(),
        })
        .expect("reviewed non-privileged configuration should be approved");
    assert_eq!(
        approved.profiles[0].review_status,
        ProfileReviewStatus::Approved
    );
}

#[test]
fn first_local_open_atomically_copies_and_validates_the_roaming_store() {
    let storage = TempStorage::new();
    let roaming = storage.path().join("roaming");
    let local = storage.path().join("local");
    fs::create_dir_all(&roaming).expect("roaming storage should be created");
    fs::create_dir_all(&local).expect("empty local storage should be created");
    let roaming_commands =
        NativeCommandHost::open(&roaming).expect("roaming command host should open");
    roaming_commands
        .create_profile(CreateProfilePayload {
            name: "Migrated endurance".to_owned(),
            primary_sim_name: "Le Mans Ultimate".to_owned(),
        })
        .expect("roaming profile should be created");
    drop(roaming_commands);

    let commands = NativeCommandHost::open_with_roaming_migration(&local, &roaming)
        .expect("valid roaming storage should migrate");
    let snapshot = commands
        .get_app_snapshot()
        .expect("migrated command host should answer");

    assert_eq!(snapshot.profiles.len(), 1);
    assert_eq!(snapshot.profiles[0].name, "Migrated endurance");
    assert!(
        fs::read_dir(local.join("profiles"))
            .expect("local profiles should be activated")
            .next()
            .is_some(),
        "the validated copy should be activated in local storage"
    );
    assert!(
        fs::read_dir(roaming.join("profiles"))
            .expect("roaming profiles should remain readable")
            .next()
            .is_some(),
        "the roaming source must remain as a recoverable backup"
    );
}

#[test]
fn invalid_roaming_documents_never_activate_local_storage() {
    let storage = TempStorage::new();
    let roaming = storage.path().join("roaming");
    let local = storage.path().join("local");
    fs::create_dir_all(roaming.join("backups")).expect("roaming backup storage should be created");
    fs::create_dir_all(&local).expect("empty local storage should be created");
    fs::write(
        roaming.join("backups").join("corrupt.json"),
        b"{ definitely not json",
    )
    .expect("invalid roaming fixture should be written");

    let error = match NativeCommandHost::open_with_roaming_migration(&local, &roaming) {
        Ok(_) => panic!("invalid roaming storage must not activate"),
        Err(error) => error,
    };

    assert_eq!(error.code, "invalid_local_state");
    assert!(
        fs::read_dir(&local)
            .expect("local storage should remain readable")
            .next()
            .is_none(),
        "the empty local destination must remain untouched"
    );
    assert!(
        roaming.join("backups").join("corrupt.json").exists(),
        "the invalid roaming source must remain recoverable"
    );
}

#[test]
fn populated_local_and_roaming_stores_are_never_merged() {
    let storage = TempStorage::new();
    let roaming = storage.path().join("roaming");
    let local = storage.path().join("local");
    let roaming_commands =
        NativeCommandHost::open(&roaming).expect("roaming command host should open");
    roaming_commands
        .create_profile(CreateProfilePayload {
            name: "Roaming profile".to_owned(),
            primary_sim_name: "Assetto Corsa Competizione".to_owned(),
        })
        .expect("roaming profile should be created");
    drop(roaming_commands);
    let local_commands = NativeCommandHost::open(&local).expect("local command host should open");
    local_commands
        .create_profile(CreateProfilePayload {
            name: "Local profile".to_owned(),
            primary_sim_name: "iRacing".to_owned(),
        })
        .expect("local profile should be created");
    drop(local_commands);

    let commands = NativeCommandHost::open_with_roaming_migration(&local, &roaming)
        .expect("a populated local store should take precedence");
    let snapshot = commands
        .get_app_snapshot()
        .expect("local command host should answer");

    assert_eq!(snapshot.profiles.len(), 1);
    assert_eq!(snapshot.profiles[0].name, "Local profile");
    assert!(
        fs::read_dir(roaming.join("profiles"))
            .expect("roaming profiles should remain readable")
            .next()
            .is_some(),
        "the conflicting roaming store should remain untouched"
    );
}
