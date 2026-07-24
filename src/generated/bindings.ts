// This file is generated from Rust. Do not edit by hand.
import { invoke } from "@tauri-apps/api/core";

export type ApplicationRequirement = "required" | "optional";

export type ConsoleVisibility = "hidden" | "visible";

export type LaunchSource = { "kind": "directExecutable", executablePath: string, } | { "kind": "steam", appId: number, };

export type ShutdownStrategy = { "kind": "closeWindows" } | { "kind": "consoleInterrupt" } | { "kind": "customStop", executablePath: string, arguments: Array<string>, } | { "kind": "forceOnly" };

export type LaunchRecipe = { source: LaunchSource, arguments: Array<string>, workingDirectory: string | null, monitoredProcess: string | null, consoleVisibility: ConsoleVisibility, elevated: boolean, startupTimeoutSeconds: number, postStartDelayMilliseconds: number, shutdownStrategy: ShutdownStrategy, };

export type ProfileApplication = { id: string, name: string, launchRecipe: LaunchRecipe, pathNeedsRepair: boolean, };

export type SupportingApplication = { application: ProfileApplication, requirement: ApplicationRequirement, keepRunning: boolean, };

export type VrLaunchMode = "openXr" | "openVr" | "oculus";

export type CloseSessionSettings = { stopSteamVr: boolean, };

export type RacingProfile = { id: string, name: string, primarySim: ProfileApplication, supportingApplications: Array<SupportingApplication>, vrEnabled: boolean, preferredVrLaunchMode: VrLaunchMode | null, closeSession: CloseSessionSettings, };

export type ProfileSummary = { id: string, name: string, primarySimName: string, };

export type ProcessIdentity = { pid: number, creationTime: string, canonicalExecutablePath: string, };

export type ProcessOwnership = "sessionOwned" | "preExisting";

export type ProcessStatus = "starting" | "running" | "runningPreExisting" | "notResponding" | "stopping" | "stopped" | "failed";

export type ApplicationProcessSnapshot = { applicationId: string, status: ProcessStatus, ownership: ProcessOwnership | null, identity: ProcessIdentity | null, };

export type AppSnapshot = { applicationName: string, foundationStatus: string, profiles: Array<ProfileSummary>, selectedProfile: RacingProfile | null, applicationProcesses: Array<ApplicationProcessSnapshot>, };

export type CommandError = { code: string, message: string, recovery: string | null, diagnosticId: string | null, };

export type CreateProfilePayload = { name: string, primarySimName: string, };

export type SaveProfilePayload = { profile: RacingProfile, };

export type ProfileIdPayload = { profileId: string, };

export type DuplicateProfilePayload = { sourceProfileId: string, name: string, };

export type ImportProfilePayload = { document: string, };

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
