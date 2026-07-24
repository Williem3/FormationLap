// This file is generated from Rust. Do not edit by hand.
import { invoke } from "@tauri-apps/api/core";

export type ProfileSummary = { id: string, name: string, primarySimName: string, };

export type AppSnapshot = { applicationName: string, foundationStatus: string, profiles: Array<ProfileSummary>, };

export function getAppSnapshot(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("get_app_snapshot");
}
