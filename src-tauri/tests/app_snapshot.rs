use formation_lap_lib::get_app_snapshot;

#[test]
fn snapshot_reports_the_secure_foundation_as_ready() {
    let snapshot = get_app_snapshot();

    assert_eq!(snapshot.application_name, "Formation Lap");
    assert_eq!(snapshot.foundation_status, "ready");
}
