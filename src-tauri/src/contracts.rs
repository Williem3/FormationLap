use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A compact Racing Profile representation for snapshots and selection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    pub primary_sim_name: String,
}

/// Authoritative native state rendered by React.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub application_name: String,
    pub foundation_status: String,
    pub profiles: Vec<ProfileSummary>,
}

impl AppSnapshot {
    pub fn foundation() -> Self {
        Self {
            application_name: "Formation Lap".to_owned(),
            foundation_status: "ready".to_owned(),
            profiles: Vec::new(),
        }
    }
}
