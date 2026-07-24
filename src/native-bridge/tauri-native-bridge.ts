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
  exportProfile,
  forceStopApplication,
  getAppSnapshot,
  importProfile,
  refreshProcesses,
  recommendApplications,
  restartApplication,
  saveProfile,
  selectProfile,
  startApplication,
  startSession,
  testGameLaunch,
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
  startSession = startSession;
  testGameLaunch = testGameLaunch;
  cancelStartup = cancelStartup;
  closeSession = closeSession;
  acceptRecovery = acceptRecovery;
  dismissRecovery = dismissRecovery;
  discoverApplications = discoverApplications;
  recommendApplications = recommendApplications;
}
