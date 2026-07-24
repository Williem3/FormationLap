import type {
  AppSnapshot,
  CreateProfilePayload,
  DuplicateProfilePayload,
  ImportProfilePayload,
  ProfileIdPayload,
  SaveProfilePayload,
} from "../generated/bindings";

export interface NativeBridge {
  getAppSnapshot(): Promise<AppSnapshot>;
  createProfile(payload: CreateProfilePayload): Promise<AppSnapshot>;
  saveProfile(payload: SaveProfilePayload): Promise<AppSnapshot>;
  selectProfile(payload: ProfileIdPayload): Promise<AppSnapshot>;
  duplicateProfile(payload: DuplicateProfilePayload): Promise<AppSnapshot>;
  deleteProfile(payload: ProfileIdPayload): Promise<AppSnapshot>;
  exportProfile(payload: ProfileIdPayload): Promise<string>;
  importProfile(payload: ImportProfilePayload): Promise<AppSnapshot>;
}
