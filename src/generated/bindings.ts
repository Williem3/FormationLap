// This file is generated from Rust. Do not edit by hand.
import { invoke } from "@tauri-apps/api/core";

export type ApplicationRequirement = "required" | "optional";

export type ConsoleVisibility = "hidden" | "visible";

export type SteamLaunchSelector = { "kind": "default" } | { "kind": "openVr" } | { "kind": "oculus" } | { "kind": "option", index: number, };

export type LaunchSource = { "kind": "directExecutable", executablePath: string, } | { "kind": "steam", appId: number, selector: SteamLaunchSelector | null, };

export type ShutdownStrategy = { "kind": "closeWindows" } | { "kind": "consoleInterrupt" } | { "kind": "customStop", executablePath: string, arguments: Array<string>, } | { "kind": "forceOnly" };

export type LaunchRecipe = { source: LaunchSource, arguments: Array<string>, workingDirectory: string | null, monitoredProcess: string | null, monitoredExecutablePath?: string | null, consoleVisibility: ConsoleVisibility, elevated: boolean, startupTimeoutSeconds: number, postStartDelayMilliseconds: number, shutdownStrategy: ShutdownStrategy, };

export type NewProfileApplication = { name: string, launchRecipe: LaunchRecipe, };

export type NewSupportingApplication = { application: NewProfileApplication, requirement: ApplicationRequirement, keepRunning: boolean, };

export type NewRacingProfile = { name: string, primarySim: NewProfileApplication, supportingApplications: Array<NewSupportingApplication>, vrEnabled: boolean, preferredVrLaunchMode: VrLaunchMode | null, closeSession: CloseSessionSettings, };

export type GameLaunchTarget = { "kind": "steam", uri: string, } | { "kind": "directExecutable", executableName: string, };

export type GameLaunchDiagnostic = { schemaVersion: number, profileName: string, vrEnabled: boolean, vrLaunchMode: VrLaunchMode | null, target: GameLaunchTarget, arguments: Array<string>, monitoredProcess: string | null, observedProcess: string, };

export type ProfileApplication = { id: string, name: string, launchRecipe: LaunchRecipe, pathNeedsRepair: boolean, };

export type SupportingApplication = { application: ProfileApplication, requirement: ApplicationRequirement, keepRunning: boolean, };

export type VrLaunchMode = "openXr" | "openVr" | "oculus";

export type CloseSessionSettings = { stopSteamVr: boolean, };

export type RacingProfile = { id: string, name: string, primarySim: ProfileApplication, supportingApplications: Array<SupportingApplication>, vrEnabled: boolean, preferredVrLaunchMode: VrLaunchMode | null, closeSession: CloseSessionSettings, };

export type ProfileReviewStatus = "approved" | "needsReview";

export type ProfileSummary = { id: string, name: string, primarySimName: string, primarySimApplicationId?: string, reviewStatus?: ProfileReviewStatus, };

export type ProcessIdentity = { pid: number, creationTime: string, canonicalExecutablePath: string, };

export type ProcessOwnership = "sessionOwned" | "preExisting";

export type ProcessStatus = "starting" | "running" | "runningPreExisting" | "notResponding" | "stopping" | "stopped" | "failed";

export type ProcessOutput = { stdout: string, stderr: string, truncated: boolean, };

export type ApplicationProcessSnapshot = { applicationId: string, status: ProcessStatus, ownership: ProcessOwnership | null, identity: ProcessIdentity | null, output: ProcessOutput | null, };

export type SessionState = "idle" | "starting" | "cancelling" | "active" | "closing" | "recoveryAvailable";

export type SessionApplicationRole = "supporting" | "primarySim";

export type SessionApplicationState = "pending" | "starting" | "running" | "runningPreExisting" | "failed" | "stopping" | "stopped" | "detached";

export type SessionEventKind = "launchFailed" | "applicationExited";

export type SessionEvent = { applicationId: string, name: string, kind: SessionEventKind, };

export type SessionSummary = { profileId: string, events: Array<SessionEvent>, };

export type SessionApplicationSnapshot = { applicationId: string, name: string, role: SessionApplicationRole, requirement: ApplicationRequirement | null, state: SessionApplicationState, };

export type SessionSnapshot = { state: SessionState, activeProfileId: string | null, applications: Array<SessionApplicationSnapshot>, summary: SessionSummary | null, };

export type ThemePreference = "system" | "light" | "dark";

export type UpdateChannel = "stable" | "beta";

export type DesktopSettings = { startWithWindows: boolean, theme: ThemePreference, reduceMotion: boolean, automaticUpdateChecks: boolean, updateChannel: UpdateChannel, };

export type DiagnosticEntry = { timestampUnixSeconds: number, event: string, outcome: string, };

export type DiagnosticExport = { schemaVersion: number, applicationVersion: string, platform: string, settings: DesktopSettings, sessionState: SessionState, profileCount: number, configuredApplicationCount: number, recentEvents: Array<DiagnosticEntry>, telemetryUpload: boolean, };

export type UpdateStatus = { "kind": "current", currentVersion: string, } | { "kind": "updateAvailable", currentVersion: string, latestVersion: string, } | { "kind": "unknown", reason: string, };

export type ApplicationUpdateSnapshot = { applicationId: string, name: string, status: UpdateStatus, informationUrl: string | null, };

export type UpdateSnapshot = { formationLap: UpdateStatus, applications: Array<ApplicationUpdateSnapshot>, lastAutomaticCheckUnixSeconds: number | null, resultDeferred: boolean, };

export type AppSnapshot = { applicationName: string, foundationStatus: string, settings: DesktopSettings, updates: UpdateSnapshot, profiles: Array<ProfileSummary>, selectedProfile: RacingProfile | null, applicationIcons?: Array<ApplicationIconSnapshot>, applicationProcesses: Array<ApplicationProcessSnapshot>, session: SessionSnapshot, };

export type ApplicationIcon = { "kind": "localData", media_type: string, data_base64: string, } | { "kind": "generic" };

export type ApplicationIconSnapshot = { applicationId: string, icon: ApplicationIcon, };

export type DiscoveredInstallation = { "kind": "directExecutable", executablePath: string, } | { "kind": "steam", appId: number, install_directory: string, };

export type CatalogPrimarySim = { id: string, name: string, steamAppId: number | null, };

export type CatalogSupportingApplication = { id: string, name: string, };

export type SupportingApplicationProfileDefaults = { arguments: Array<string>, consoleVisibility: ConsoleVisibility, elevated: boolean, startupTimeoutSeconds: number, postStartDelayMilliseconds: number, shutdownStrategy: ShutdownStrategy, requirement: ApplicationRequirement, keepRunning: boolean, };

export type DiscoveredPrimarySim = { id: string, name: string, installation: DiscoveredInstallation, icon: ApplicationIcon, };

export type DiscoveredSupportingApplication = { id: string, name: string, installation: DiscoveredInstallation, profileDefaults: SupportingApplicationProfileDefaults, icon: ApplicationIcon, };

export type DiscoverySnapshot = { primarySims: Array<CatalogPrimarySim>, supportingApplications: Array<CatalogSupportingApplication>, installedPrimarySims: Array<DiscoveredPrimarySim>, installedSupportingApplications: Array<DiscoveredSupportingApplication>, };

export type CompatibilityRank = "recommended" | "compatible";

export type CatalogUpdateProvider = { "kind": "githubReleases", repository: string, } | { "kind": "winget", packageId: string, } | { "kind": "officialPage", url: string, };

export type SupportingApplicationRecommendation = { id: string, name: string, rank: CompatibilityRank, updateProvider: CatalogUpdateProvider | null, };

export type CommandError = { code: string, message: string, recovery: string | null, diagnosticId: string | null, };

export type CreateProfilePayload = { profile: NewRacingProfile, };

export type SaveProfilePayload = { profile: RacingProfile, };

export type ProfileIdPayload = { profileId: string, };

export type DuplicateProfilePayload = { sourceProfileId: string, name: string, };

export type ImportProfilePayload = { document: string, };

export type ApproveProfilePayload = { profileId: string, configurationReviewed: boolean, approvedPrivilegedApplicationIds: Array<string>, };

export type ApplicationTargetPayload = { profileId: string, applicationId: string, };

export type ExitApplicationPayload = { applicationId: string, preExistingConfirmed: boolean, };

export type ForceStopApplicationPayload = { applicationId: string, preExistingConfirmed: boolean, forceConfirmed: boolean, };

export type RestartApplicationPayload = { profileId: string, applicationId: string, preExistingConfirmed: boolean, };

export type PrimarySimIdPayload = { primarySimId: string, };

export type QuitDisposition = "closeSession" | "leaveApplicationsRunning";

export type QuitPayload = { disposition: QuitDisposition, };

export type UpdateSettingsPayload = { settings: DesktopSettings, };

export function getAppSnapshot(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("get_app_snapshot");
}

export function createProfile(payload: CreateProfilePayload): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("create_profile", { payload });
}

export function saveProfile(payload: SaveProfilePayload): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("save_profile", { payload });
}

export function selectProfile(payload: ProfileIdPayload): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("select_profile", { payload });
}

export function duplicateProfile(payload: DuplicateProfilePayload): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("duplicate_profile", { payload });
}

export function deleteProfile(payload: ProfileIdPayload): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("delete_profile", { payload });
}

export function exportProfile(payload: ProfileIdPayload): Promise<string> {
  return invoke<string>("export_profile", { payload });
}

export function importProfile(payload: ImportProfilePayload): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("import_profile", { payload });
}

export function approveProfile(payload: ApproveProfilePayload): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("approve_profile", { payload });
}

export function pickExecutablePath(initialPath?: string | null): Promise<string | null> {
  return invoke<string | null>("pick_executable_path", { initialPath });
}

export function startApplication(payload: ApplicationTargetPayload): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("start_application", { payload });
}

export function refreshProcesses(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("refresh_processes");
}

export function exitApplication(payload: ExitApplicationPayload): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("exit_application", { payload });
}

export function forceStopApplication(payload: ForceStopApplicationPayload): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("force_stop_application", { payload });
}

export function restartApplication(payload: RestartApplicationPayload): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("restart_application", { payload });
}

export function startSession(payload: ProfileIdPayload): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("start_session", { payload });
}

export function testGameLaunch(payload: ProfileIdPayload): Promise<GameLaunchDiagnostic> {
  return invoke<GameLaunchDiagnostic>("test_game_launch", { payload });
}

export function cancelStartup(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("cancel_startup");
}

export function closeSession(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("close_session");
}

export function requestQuit(payload: QuitPayload): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("request_quit", { payload });
}

export function updateSettings(payload: UpdateSettingsPayload): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("update_settings", { payload });
}

export function checkUpdates(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("check_updates");
}

export function installFormationLapUpdate(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("install_formation_lap_update");
}

export function exportDiagnostics(): Promise<DiagnosticExport> {
  return invoke<DiagnosticExport>("export_diagnostics");
}

export function acceptRecovery(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("accept_recovery");
}

export function dismissRecovery(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("dismiss_recovery");
}

export function discoverApplications(): Promise<DiscoverySnapshot> {
  return invoke<DiscoverySnapshot>("discover_applications");
}

export function recommendApplications(payload: PrimarySimIdPayload): Promise<SupportingApplicationRecommendation[]> {
  return invoke<SupportingApplicationRecommendation[]>("recommend_applications", { payload });
}
