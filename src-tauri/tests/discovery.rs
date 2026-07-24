use formation_lap_lib::{
    AppCommand, CommandOutcome, DiscoveredInstallation, FormationLapCore, TargetedDiscoverySources,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{self, Command},
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
            "formation-lap-discovery-test-{}-{timestamp}-{unique}",
            process::id()
        ));
        fs::create_dir_all(&path).expect("temporary discovery storage should be created");
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
fn bundled_catalog_returns_exactly_the_reviewed_primary_sims() {
    let storage = TempStorage::new();
    let mut core =
        FormationLapCore::open(storage.path()).expect("FormationLapCore should open its catalog");

    let discovery = match core
        .execute(AppCommand::DiscoverApplications)
        .expect("bundled discovery should complete")
    {
        CommandOutcome::ApplicationsDiscovered { discovery } => discovery,
        other => panic!("expected bundled discovery, got {other:?}"),
    };

    assert_eq!(
        discovery
            .primary_sims
            .iter()
            .map(|sim| (sim.id.as_str(), sim.name.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("iracing", "iRacing"),
            ("assetto-corsa", "Assetto Corsa"),
            ("assetto-corsa-competizione", "Assetto Corsa Competizione"),
            ("assetto-corsa-evo", "Assetto Corsa EVO"),
            ("automobilista-2", "Automobilista 2"),
            ("rfactor-2", "rFactor 2"),
            ("le-mans-ultimate", "Le Mans Ultimate"),
            ("raceroom", "RaceRoom Racing Experience"),
            ("ea-sports-wrc", "EA SPORTS WRC"),
            ("dirt-rally-2", "DiRT Rally 2.0"),
        ]
    );
}

#[test]
fn bundled_catalog_returns_exactly_the_reviewed_supporting_applications() {
    let storage = TempStorage::new();
    let mut core =
        FormationLapCore::open(storage.path()).expect("FormationLapCore should open its catalog");

    let discovery = match core
        .execute(AppCommand::DiscoverApplications)
        .expect("bundled discovery should complete")
    {
        CommandOutcome::ApplicationsDiscovered { discovery } => discovery,
        other => panic!("expected bundled discovery, got {other:?}"),
    };

    assert_eq!(
        discovery
            .supporting_applications
            .iter()
            .map(|application| (application.id.as_str(), application.name.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("simhub", "SimHub"),
            ("crew-chief", "Crew Chief"),
            ("trading-paints", "Trading Paints"),
            ("garage-61", "Garage 61"),
            ("racelab", "RaceLab"),
            ("ioverlay", "iOverlay"),
            ("go-fast", "Go Fast"),
            ("steamvr", "SteamVR"),
            ("lmuffb", "LMUFFB"),
        ]
    );
}

#[test]
fn catalog_validator_rejects_duplicate_sim_ids_with_an_actionable_location() {
    let temporary = TempStorage::new();
    let sims_path = temporary.path().join("duplicate-sims.json");
    let applications_path = temporary.path().join("applications.json");
    fs::write(
        &sims_path,
        r#"{
          "schemaVersion": 1,
          "sims": [
            { "id": "iracing", "name": "iRacing" },
            { "id": "iracing", "name": "Duplicate iRacing" }
          ]
        }"#,
    )
    .expect("invalid sim fixture should be written");
    fs::write(
        &applications_path,
        r#"{
          "schemaVersion": 1,
          "applications": [
            { "id": "simhub", "name": "SimHub" }
          ]
        }"#,
    )
    .expect("valid application fixture should be written");

    let output = Command::new(env!(
        "CARGO_BIN_EXE_validate-catalog",
        "catalog validator should be available to CI"
    ))
    .args([
        "--sims",
        sims_path
            .to_str()
            .expect("temporary sim path should be Unicode"),
        "--applications",
        applications_path
            .to_str()
            .expect("temporary application path should be Unicode"),
    ])
    .output()
    .expect("catalog validator should execute");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8(output.stderr).expect("validator error should be UTF-8"),
        "duplicate sim id 'iracing' at sims[1].id; first declared at sims[0].id\n"
    );
}

#[test]
fn catalog_validator_rejects_duplicate_steam_app_ids_with_an_actionable_location() {
    let temporary = TempStorage::new();
    let sims_path = temporary.path().join("duplicate-steam-app-ids.json");
    let applications_path = temporary.path().join("applications.json");
    fs::write(
        &sims_path,
        r#"{
          "schemaVersion": 1,
          "sims": [
            { "id": "iracing", "name": "iRacing", "steamAppId": 266410 },
            { "id": "not-iracing", "name": "Not iRacing", "steamAppId": 266410 }
          ]
        }"#,
    )
    .expect("invalid Steam App ID fixture should be written");
    fs::write(
        &applications_path,
        r#"{
          "schemaVersion": 1,
          "applications": [
            { "id": "simhub", "name": "SimHub" }
          ]
        }"#,
    )
    .expect("valid application fixture should be written");

    let output = Command::new(env!(
        "CARGO_BIN_EXE_validate-catalog",
        "catalog validator should be available to CI"
    ))
    .args([
        "--sims",
        sims_path
            .to_str()
            .expect("temporary sim path should be Unicode"),
        "--applications",
        applications_path
            .to_str()
            .expect("temporary application path should be Unicode"),
    ])
    .output()
    .expect("catalog validator should execute");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8(output.stderr).expect("validator error should be UTF-8"),
        "duplicate Steam App ID 266410 at sims[1].steamAppId; first declared at sims[0].steamAppId\n"
    );
}

#[test]
fn steam_discovery_follows_declared_libraries_and_omits_missing_installations() {
    let storage = TempStorage::new();
    let steam_root = storage.path().join("Steam");
    let second_library = storage.path().join("Second Library");
    let steamapps = steam_root.join("steamapps");
    let second_steamapps = second_library.join("steamapps");
    fs::create_dir_all(steamapps.join("common").join("assettocorsa"))
        .expect("first Steam installation should be created");
    fs::create_dir_all(second_steamapps.join("common").join("Le Mans Ultimate"))
        .expect("second Steam installation should be created");
    let escaped_steam_root = steam_root.to_string_lossy().replace('\\', "\\\\");
    let escaped_second_library = second_library.to_string_lossy().replace('\\', "\\\\");
    fs::write(
        steamapps.join("libraryfolders.vdf"),
        format!(
            r#""libraryfolders"
{{
  "0"
  {{
    "path" "{escaped_steam_root}"
  }}
  "1"
  {{
    "path" "{escaped_second_library}"
  }}
}}"#
        ),
    )
    .expect("Steam library declaration should be written");
    fs::write(
        steamapps.join("appmanifest_244210.acf"),
        r#""AppState"
{
  "appid" "244210"
  "installdir" "assettocorsa"
}"#,
    )
    .expect("Assetto Corsa manifest should be written");
    fs::write(
        second_steamapps.join("appmanifest_2399420.acf"),
        r#""AppState"
{
  "appid" "2399420"
  "installdir" "Le Mans Ultimate"
}"#,
    )
    .expect("Le Mans Ultimate manifest should be written");
    fs::write(
        second_steamapps.join("appmanifest_805550.acf"),
        r#""AppState"
{
  "appid" "805550"
  "installdir" "Missing ACC"
}"#,
    )
    .expect("stale missing-installation manifest should be written");

    let mut core = FormationLapCore::open_with_discovery_sources(
        storage.path(),
        TargetedDiscoverySources {
            steam_roots: vec![steam_root],
        },
    )
    .expect("FormationLapCore should open with targeted Steam roots");
    let discovery = match core
        .execute(AppCommand::DiscoverApplications)
        .expect("targeted Steam discovery should complete")
    {
        CommandOutcome::ApplicationsDiscovered { discovery } => discovery,
        other => panic!("expected local discovery, got {other:?}"),
    };

    assert_eq!(
        discovery
            .installed_primary_sims
            .iter()
            .map(|sim| {
                let app_id = match &sim.installation {
                    DiscoveredInstallation::Steam { app_id, .. } => *app_id,
                    other => panic!("expected Steam installation, got {other:?}"),
                };
                (sim.id.as_str(), app_id)
            })
            .collect::<Vec<_>>(),
        vec![("assetto-corsa", 244210), ("le-mans-ultimate", 2399420)]
    );
}
