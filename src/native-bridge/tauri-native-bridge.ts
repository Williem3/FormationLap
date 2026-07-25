import {
  acceptRecovery,
  approveProfile,
  cancelStartup,
  checkUpdates,
  closeSession,
  createProfile,
  deleteProfile,
  dismissRecovery,
  discoverApplications,
  duplicateProfile,
  exitApplication,
  exportDiagnostics,
  exportProfile,
  forceStopApplication,
  getAppSnapshot,
  importProfile,
  installFormationLapUpdate,
  pickExecutablePath,
  refreshProcesses,
  recommendApplications,
  requestQuit,
  restartApplication,
  saveProfile,
  selectProfile,
  startApplication,
  startSession,
  testGameLaunch,
  updateSettings,
} from "../generated/bindings";
import { listen } from "@tauri-apps/api/event";
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
  approveProfile = approveProfile;
  pickExecutablePath = pickExecutablePath;
  startApplication = startApplication;
  refreshProcesses = refreshProcesses;
  exitApplication = exitApplication;
  forceStopApplication = forceStopApplication;
  restartApplication = restartApplication;
  startSession = startSession;
  testGameLaunch = testGameLaunch;
  cancelStartup = cancelStartup;
  closeSession = closeSession;
  requestQuit = requestQuit;
  listenForQuitRequest(listener: () => void): Promise<() => void> {
    return listen("formation-lap://quit-requested", listener);
  }
  updateSettings = updateSettings;
  checkUpdates = checkUpdates;
  installFormationLapUpdate = installFormationLapUpdate;
  exportDiagnostics = exportDiagnostics;
  acceptRecovery = acceptRecovery;
  dismissRecovery = dismissRecovery;
  discoverApplications = discoverApplications;
  recommendApplications = recommendApplications;
}
