use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

fn new_application_id() -> String {
    Uuid::new_v4().to_string()
}

fn path_needs_repair_by_default() -> bool {
    true
}

/// Whether a Supporting Application may block the Primary Sim.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum ApplicationRequirement {
    Required,
    Optional,
}

/// Whether a launched console should be shown to the user.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum ConsoleVisibility {
    Hidden,
    Visible,
}

/// Portable source choice for one Launch Recipe.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum LaunchSource {
    DirectExecutable {
        #[serde(rename = "executablePath")]
        #[ts(rename = "executablePath")]
        executable_path: String,
    },
    Steam {
        #[serde(rename = "appId")]
        #[ts(rename = "appId")]
        app_id: u32,
    },
}

/// User-configured graceful shutdown request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum ShutdownStrategy {
    CloseWindows,
    ConsoleInterrupt,
    CustomStop {
        #[serde(rename = "executablePath")]
        #[ts(rename = "executablePath")]
        executable_path: String,
        arguments: Vec<String>,
    },
    ForceOnly,
}

/// Saved instructions for launching and identifying one application.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct LaunchRecipe {
    pub source: LaunchSource,
    pub arguments: Vec<String>,
    pub working_directory: Option<String>,
    pub monitored_process: Option<String>,
    pub console_visibility: ConsoleVisibility,
    pub elevated: bool,
    pub startup_timeout_seconds: u32,
    pub post_start_delay_milliseconds: u32,
    pub shutdown_strategy: ShutdownStrategy,
}

impl Default for LaunchRecipe {
    fn default() -> Self {
        Self {
            source: LaunchSource::DirectExecutable {
                executable_path: String::new(),
            },
            arguments: Vec::new(),
            working_directory: None,
            monitored_process: None,
            console_visibility: ConsoleVisibility::Hidden,
            elevated: false,
            startup_timeout_seconds: 30,
            post_start_delay_milliseconds: 0,
            shutdown_strategy: ShutdownStrategy::CloseWindows,
        }
    }
}

/// One Primary Sim or Supporting Application saved in a Racing Profile.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ProfileApplication {
    #[serde(default = "new_application_id")]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub launch_recipe: LaunchRecipe,
    #[serde(default = "path_needs_repair_by_default")]
    pub path_needs_repair: bool,
}

/// A Supporting Application plus its Session policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SupportingApplication {
    pub application: ProfileApplication,
    pub requirement: ApplicationRequirement,
    pub keep_running: bool,
}

/// Game-specific virtual-reality path preferred by a Racing Profile.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum VrLaunchMode {
    OpenXr,
    OpenVr,
    Oculus,
}

/// Session cleanup choices owned by one Racing Profile.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CloseSessionSettings {
    pub stop_steam_vr: bool,
}

/// Complete editable Racing Profile state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct RacingProfile {
    pub id: String,
    pub name: String,
    pub primary_sim: ProfileApplication,
    pub supporting_applications: Vec<SupportingApplication>,
    pub vr_enabled: bool,
    pub preferred_vr_launch_mode: Option<VrLaunchMode>,
    pub close_session: CloseSessionSettings,
}

/// A compact Racing Profile representation for snapshots and selection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    pub primary_sim_name: String,
}

/// Stable identity for one observed Windows process.
///
/// Creation time is an opaque decimal 100-nanosecond Windows timestamp.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ProcessIdentity {
    pub pid: u32,
    pub creation_time: String,
    pub canonical_executable_path: String,
}

/// Whether Formation Lap started a Process or merely observed it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum ProcessOwnership {
    SessionOwned,
    PreExisting,
}

/// User-visible lifecycle state for a configured application.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum ProcessStatus {
    Starting,
    Running,
    RunningPreExisting,
    NotResponding,
    Stopping,
    Stopped,
    Failed,
}

/// Bounded stdout and stderr tail captured for one launched Process.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ProcessOutput {
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
}

/// Authoritative lifecycle state for one configured application.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ApplicationProcessSnapshot {
    pub application_id: String,
    pub status: ProcessStatus,
    pub ownership: Option<ProcessOwnership>,
    pub identity: Option<ProcessIdentity>,
    pub output: Option<ProcessOutput>,
}

/// Authoritative native state rendered by React.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub application_name: String,
    pub foundation_status: String,
    pub profiles: Vec<ProfileSummary>,
    pub selected_profile: Option<RacingProfile>,
    pub application_processes: Vec<ApplicationProcessSnapshot>,
}

impl AppSnapshot {
    pub fn foundation() -> Self {
        Self {
            application_name: "Formation Lap".to_owned(),
            foundation_status: "ready".to_owned(),
            profiles: Vec::new(),
            selected_profile: None,
            application_processes: Vec::new(),
        }
    }
}
