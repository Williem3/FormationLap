import {
  acceptRecovery,
  cancelStartup,
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
  exportDiagnostics = exportDiagnostics;
  acceptRecovery = acceptRecovery;
  dismissRecovery = dismissRecovery;
  discoverApplications = discoverApplications;
  recommendApplications = recommendApplications;
}
