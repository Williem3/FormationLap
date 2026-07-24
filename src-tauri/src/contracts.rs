use serde::Serialize;
use ts_rs::TS;

/// Authoritative native state rendered by the M1 shell.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub application_name: String,
    pub foundation_status: String,
}

impl AppSnapshot {
    pub fn foundation() -> Self {
        Self {
            application_name: "Formation Lap".to_owned(),
            foundation_status: "ready".to_owned(),
        }
    }
}
