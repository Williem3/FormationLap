use crate::AppSnapshot;

/// Returns the smallest authoritative snapshot needed by the foundation shell.
#[tauri::command]
pub fn get_app_snapshot() -> AppSnapshot {
    AppSnapshot::foundation()
}
