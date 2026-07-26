import type {
  ApplicationProcessSnapshot,
  AppSnapshot,
  DesktopSettings,
  ProfileApplication,
  RacingProfile,
  SessionSnapshot,
  UpdateSnapshot,
} from "../generated/bindings";
import { idleSessionSnapshot } from "../session/session-snapshot";

type AppSnapshotOverrides = Omit<
  Partial<AppSnapshot>,
  "settings" | "updates" | "session"
> & {
  settings?: Partial<DesktopSettings>;
  updates?: Partial<UpdateSnapshot>;
  session?: Partial<SessionSnapshot>;
};

export function createProfileApplication(
  overrides: Partial<ProfileApplication> = {},
): ProfileApplication {
  return {
    id: "application-test",
    name: "Test application",
    launchRecipe: {
      source: {
        kind: "directExecutable",
        executablePath: String.raw`C:\Fixtures\test-application.exe`,
      },
      arguments: [],
      workingDirectory: String.raw`C:\Fixtures`,
      monitoredProcess: null,
      monitoredExecutablePath: null,
      consoleVisibility: "hidden",
      elevated: false,
      startupTimeoutSeconds: 30,
      postStartDelayMilliseconds: 0,
      shutdownStrategy: { kind: "closeWindows" },
    },
    pathNeedsRepair: false,
    ...overrides,
  };
}

export function createRacingProfile(
  overrides: Partial<RacingProfile> = {},
): RacingProfile {
  return {
    id: "profile-test",
    name: "Test Racing Profile",
    primarySim: createProfileApplication({
      id: "primary-sim-test",
      name: "Test Primary Sim",
    }),
    supportingApplications: [],
    vrEnabled: false,
    preferredVrLaunchMode: null,
    closeSession: { stopSteamVr: false },
    ...overrides,
  };
}

export function createApplicationProcessSnapshot(
  overrides: Partial<ApplicationProcessSnapshot> = {},
): ApplicationProcessSnapshot {
  return {
    applicationId: "application-test",
    status: "running",
    ownership: "sessionOwned",
    identity: {
      pid: 4242,
      creationTime: "133822233344455566",
      canonicalExecutablePath: String.raw`C:\Fixtures\test-application.exe`,
    },
    output: null,
    ...overrides,
  };
}

export function createAppSnapshot(
  overrides: AppSnapshotOverrides = {},
): AppSnapshot {
  const {
    settings: settingsOverrides,
    updates: updateOverrides,
    session: sessionOverrides,
    ...snapshotOverrides
  } = overrides;
  const settings: DesktopSettings = {
    startWithWindows: false,
    theme: "system",
    reduceMotion: false,
    automaticUpdateChecks: true,
    updateChannel: "stable",
    ...settingsOverrides,
  };
  const updates: UpdateSnapshot = {
    formationLap: { kind: "unknown", reason: "Not checked yet." },
    applications: [],
    lastAutomaticCheckUnixSeconds: null,
    resultDeferred: false,
    ...updateOverrides,
  };
  const session: SessionSnapshot = {
    ...idleSessionSnapshot(),
    ...sessionOverrides,
  };

  return {
    applicationName: "Formation Lap",
    foundationStatus: "ready",
    settings,
    updates,
    session,
    applicationProcesses: [],
    profiles: [],
    selectedProfile: null,
    ...snapshotOverrides,
  };
}
