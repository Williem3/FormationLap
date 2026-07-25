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

/// Explicit Steam launch-option selector that never requests the choice dialog.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum SteamLaunchSelector {
    Default,
    OpenVr,
    Oculus,
    Option { index: u8 },
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
        #[serde(default)]
        selector: Option<SteamLaunchSelector>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitored_executable_path: Option<String>,
    pub console_visibility: ConsoleVisibility,
    pub elevated: bool,
    pub startup_timeout_seconds: u32,
    pub post_start_delay_milliseconds: u32,
    pub shutdown_strategy: ShutdownStrategy,
}

/// Sanitized launch target included in a local Test Game Launch report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum GameLaunchTarget {
    Steam {
        uri: String,
    },
    DirectExecutable {
        #[serde(rename = "executableName")]
        #[ts(rename = "executableName")]
        executable_name: String,
    },
}

/// Copyable, local-only evidence from a Test Game Launch.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct GameLaunchDiagnostic {
    pub schema_version: u32,
    pub profile_name: String,
    pub vr_enabled: bool,
    pub vr_launch_mode: Option<VrLaunchMode>,
    pub target: GameLaunchTarget,
    pub arguments: Vec<String>,
    pub monitored_process: Option<String>,
    pub observed_process: String,
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
            monitored_executable_path: None,
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

/// Whether native authorization permits one Racing Profile to start a Session.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum ProfileReviewStatus {
    #[default]
    Approved,
    NeedsReview,
}

impl ProfileReviewStatus {
    fn is_approved(&self) -> bool {
        self == &Self::Approved
    }
}

/// A compact Racing Profile representation for snapshots and selection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    pub primary_sim_name: String,
    #[serde(default, skip_serializing_if = "ProfileReviewStatus::is_approved")]
    pub review_status: ProfileReviewStatus,
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

/// One reviewed Primary Sim from the bundled Curated Catalog.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CatalogPrimarySim {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub steam_app_id: Option<u32>,
}

/// One reviewed Supporting Application from the bundled Curated Catalog.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CatalogSupportingApplication {
    pub id: String,
    pub name: String,
}

/// A locally verified way to launch one discovered application.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum DiscoveredInstallation {
    DirectExecutable {
        #[serde(rename = "executablePath")]
        #[ts(rename = "executablePath")]
        executable_path: String,
    },
    Steam {
        #[serde(rename = "appId")]
        #[ts(rename = "appId")]
        app_id: u32,
        install_directory: String,
    },
}

/// Local-only icon data or Formation Lap's generic visual fallback.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum ApplicationIcon {
    LocalData {
        media_type: String,
        data_base64: String,
    },
    #[default]
    Generic,
}

/// One curated Primary Sim found at a targeted local source.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DiscoveredPrimarySim {
    pub id: String,
    pub name: String,
    pub installation: DiscoveredInstallation,
    #[serde(default)]
    pub icon: ApplicationIcon,
}

/// One curated Supporting Application found at a targeted local source.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DiscoveredSupportingApplication {
    pub id: String,
    pub name: String,
    pub installation: DiscoveredInstallation,
    #[serde(default)]
    pub icon: ApplicationIcon,
}

/// Curated compatibility strength between a sim and Supporting Application.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum CompatibilityRank {
    Recommended,
    Compatible,
}

/// Notification-only update metadata bundled with one catalog entry.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum CatalogUpdateProvider {
    #[serde(rename = "githubReleases")]
    #[ts(rename = "githubReleases")]
    GitHubReleases {
        repository: String,
    },
    Winget {
        #[serde(rename = "packageId")]
        #[ts(rename = "packageId")]
        package_id: String,
    },
    OfficialPage {
        url: String,
    },
}

/// One compatibility-ranked suggestion for a selected Primary Sim.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SupportingApplicationRecommendation {
    pub id: String,
    pub name: String,
    pub rank: CompatibilityRank,
    pub update_provider: Option<CatalogUpdateProvider>,
}

/// Curated and locally discovered applications available to profile flows.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DiscoverySnapshot {
    pub primary_sims: Vec<CatalogPrimarySim>,
    pub supporting_applications: Vec<CatalogSupportingApplication>,
    #[serde(default)]
    pub installed_primary_sims: Vec<DiscoveredPrimarySim>,
    #[serde(default)]
    pub installed_supporting_applications: Vec<DiscoveredSupportingApplication>,
}

/// Authoritative lifecycle state for the one possible Active Session.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum SessionState {
    #[default]
    Idle,
    Starting,
    Cancelling,
    Active,
    Closing,
    RecoveryAvailable,
}

/// Native window action chosen from authoritative Session state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum WindowCloseAction {
    Exit,
    HideToTray,
}

/// Explicit choice made before Formation Lap exits with a live Session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum QuitDisposition {
    CloseSession,
    LeaveApplicationsRunning,
}

/// Native application action after FormationLapCore applies Quit policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum QuitAction {
    ExitNow,
    WaitForSessionClose,
}

/// Visual theme selected for Formation Lap.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

/// Local desktop preferences shared by the native host and React.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DesktopSettings {
    pub start_with_windows: bool,
    pub theme: ThemePreference,
    pub reduce_motion: bool,
    #[serde(default = "default_automatic_update_checks")]
    pub automatic_update_checks: bool,
    #[serde(default)]
    pub update_channel: UpdateChannel,
}

impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            start_with_windows: false,
            theme: ThemePreference::System,
            reduce_motion: false,
            automatic_update_checks: true,
            update_channel: UpdateChannel::Stable,
        }
    }
}

const fn default_automatic_update_checks() -> bool {
    true
}

/// One sanitized entry from Formation Lap's bounded local diagnostic log.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DiagnosticEntry {
    #[ts(type = "number")]
    pub timestamp_unix_seconds: u64,
    pub event: String,
    pub outcome: String,
}

/// Copyable local-only evidence for support and troubleshooting.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DiagnosticExport {
    pub schema_version: u32,
    pub application_version: String,
    pub platform: String,
    pub settings: DesktopSettings,
    pub session_state: SessionState,
    pub profile_count: usize,
    pub configured_application_count: usize,
    pub recent_events: Vec<DiagnosticEntry>,
    pub telemetry_upload: bool,
}

/// Signed Formation Lap release channel selected by the user.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum UpdateChannel {
    #[default]
    Stable,
    Beta,
}

/// Why an update check was requested.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdateCheckTrigger {
    Automatic,
    Manual,
}

/// Comparable update knowledge. Unknown is explicit and never guessed.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum UpdateStatus {
    Current {
        #[serde(rename = "currentVersion")]
        #[ts(rename = "currentVersion")]
        current_version: String,
    },
    UpdateAvailable {
        #[serde(rename = "currentVersion")]
        #[ts(rename = "currentVersion")]
        current_version: String,
        #[serde(rename = "latestVersion")]
        #[ts(rename = "latestVersion")]
        latest_version: String,
    },
    Unknown {
        reason: String,
    },
}

impl Default for UpdateStatus {
    fn default() -> Self {
        Self::Unknown {
            reason: "Not checked yet.".to_owned(),
        }
    }
}

/// Notification-only update knowledge for one configured application.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ApplicationUpdateSnapshot {
    pub application_id: String,
    pub name: String,
    pub status: UpdateStatus,
    pub information_url: Option<String>,
}

/// Update advice rendered by React from authoritative native state.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UpdateSnapshot {
    pub formation_lap: UpdateStatus,
    pub applications: Vec<ApplicationUpdateSnapshot>,
    #[ts(type = "number | null")]
    pub last_automatic_check_unix_seconds: Option<u64>,
    pub result_deferred: bool,
}

/// Direct-provider work that may begin after core policy approves a check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateCheckPlan {
    pub request_id: String,
    pub channel: UpdateChannel,
    pub trigger: UpdateCheckTrigger,
    pub applications: Vec<ApplicationUpdateTarget>,
}

/// One configured application whose direct provider may be checked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationUpdateTarget {
    pub application_id: String,
    pub name: String,
    pub executable_path: Option<String>,
    pub provider: Option<CatalogUpdateProvider>,
}

/// Provider results returned to core after direct checks complete.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateCheckResult {
    pub request_id: String,
    pub formation_lap: UpdateStatus,
    pub applications: Vec<ApplicationUpdateSnapshot>,
}

/// Core decision made before any update network activity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdateCheckDecision {
    Planned(UpdateCheckPlan),
    Disabled,
    NotDue,
    Deferred,
    InProgress,
}

/// Core decision made before the signed Formation Lap updater may install.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormationLapInstallDecision {
    Ready { latest_version: String },
    NoUpdate,
    Deferred,
}

/// Placement of one application in a Session's immutable Startup Sequence.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum SessionApplicationRole {
    Supporting,
    PrimarySim,
}

/// Session-specific state rendered by one Formation Rail node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum SessionApplicationState {
    Pending,
    Starting,
    Running,
    RunningPreExisting,
    Failed,
    Stopping,
    Stopped,
    Detached,
}

/// Quietly recorded Session event surfaced only after racing ends.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub enum SessionEventKind {
    LaunchFailed,
    ApplicationExited,
}

/// One noteworthy lifecycle outcome from the most recent Session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SessionEvent {
    pub application_id: String,
    pub name: String,
    pub kind: SessionEventKind,
}

/// Non-disruptive summary made available only after the Session is Idle.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SessionSummary {
    pub profile_id: String,
    pub events: Vec<SessionEvent>,
}

/// One ordered node in the authoritative Session snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SessionApplicationSnapshot {
    pub application_id: String,
    pub name: String,
    pub role: SessionApplicationRole,
    pub requirement: Option<ApplicationRequirement>,
    pub state: SessionApplicationState,
}

/// Authoritative Session state rendered by React.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub state: SessionState,
    pub active_profile_id: Option<String>,
    pub applications: Vec<SessionApplicationSnapshot>,
    pub summary: Option<SessionSummary>,
}

/// Authoritative native state rendered by React.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub application_name: String,
    pub foundation_status: String,
    pub settings: DesktopSettings,
    pub updates: UpdateSnapshot,
    pub profiles: Vec<ProfileSummary>,
    pub selected_profile: Option<RacingProfile>,
    pub application_processes: Vec<ApplicationProcessSnapshot>,
    pub session: SessionSnapshot,
}

impl AppSnapshot {
    pub fn foundation() -> Self {
        Self {
            application_name: "Formation Lap".to_owned(),
            foundation_status: "ready".to_owned(),
            settings: DesktopSettings::default(),
            updates: UpdateSnapshot::default(),
            profiles: Vec::new(),
            selected_profile: None,
            application_processes: Vec::new(),
            session: SessionSnapshot::default(),
        }
    }
}
