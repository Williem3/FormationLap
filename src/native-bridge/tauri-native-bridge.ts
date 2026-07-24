import {
  createProfile,
  deleteProfile,
  duplicateProfile,
  exitApplication,
  exportProfile,
  forceStopApplication,
  getAppSnapshot,
  importProfile,
  refreshProcesses,
  restartApplication,
  saveProfile,
  selectProfile,
  startApplication,
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
  startApplication = startApplication;
  refreshProcesses = refreshProcesses;
  exitApplication = exitApplication;
  forceStopApplication = forceStopApplication;
  restartApplication = restartApplication;
}
