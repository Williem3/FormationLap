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

export type AppSnapshot = { applicationName: string, foundationStatus: string, profiles: Array<ProfileSummary>, selectedProfile: RacingProfile | null, };

export function getAppSnapshot(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("get_app_snapshot");
}
