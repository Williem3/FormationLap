use formation_lap_lib::{AppCommand, CommandOutcome, FormationLapCore, ProfileSummary};
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
    let CommandOutcome::ProfileCreated { profile_id } = outcome;

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
