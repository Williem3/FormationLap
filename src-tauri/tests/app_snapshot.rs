use formation_lap_lib::NativeCommandHost;
use std::{
    fs, process,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn snapshot_reports_the_secure_foundation_as_ready() {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after the Unix epoch")
        .as_nanos();
    let storage = std::env::temp_dir().join(format!(
        "formation-lap-snapshot-{}-{timestamp}",
        process::id()
    ));
    fs::create_dir_all(&storage).expect("temporary snapshot storage should be created");
    let commands = NativeCommandHost::open(&storage).expect("native command host should open");
    let snapshot = commands
        .get_app_snapshot()
        .expect("authoritative snapshot should be returned");

    assert_eq!(snapshot.application_name, "Formation Lap");
    assert_eq!(snapshot.foundation_status, "ready");

    drop(commands);
    let _ = fs::remove_dir_all(storage);
}
