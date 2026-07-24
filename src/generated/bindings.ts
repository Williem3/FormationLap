// This file is generated from Rust. Do not edit by hand.
import { invoke } from "@tauri-apps/api/core";

export type AppSnapshot = { applicationName: string, foundationStatus: string, };

export function getAppSnapshot(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("get_app_snapshot");
}
