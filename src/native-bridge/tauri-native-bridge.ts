import {
  createProfile,
  deleteProfile,
  duplicateProfile,
  exportProfile,
  getAppSnapshot,
  importProfile,
  saveProfile,
  selectProfile,
} from "../generated/bindings";
import type { NativeBridge } from "./native-bridge";

export class TauriNativeBridge implements NativeBridge {
  getAppSnapshot = getAppSnapshot;
  createProfile = createProfile;
  saveProfile = saveProfile;
  selectProfile = selectProfile;
  duplicateProfile = duplicateProfile;
  deleteProfile = deleteProfile;
  exportProfile = exportProfile;
  importProfile = importProfile;
}
