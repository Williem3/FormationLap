import type {
  AppSnapshot,
  ProfileApplication,
  SessionApplicationSnapshot,
  SessionState,
} from "../generated/bindings";
import { InMemoryNativeBridge } from "../native-bridge/in-memory-native-bridge";
import { idleSessionSnapshot } from "../session/session-snapshot";

function application(
  id: string,
  name: string,
  executablePath: string,
): ProfileApplication {
  return {
    id,
    name,
    launchRecipe: {
      source: { kind: "directExecutable", executablePath },
      arguments: [],
      workingDirectory: executablePath.slice(
        0,
        Math.max(0, executablePath.lastIndexOf("\\")),
      ),
      monitoredProcess: null,
      monitoredExecutablePath: null,
      consoleVisibility: "hidden",
      elevated: false,
      startupTimeoutSeconds: 30,
      postStartDelayMilliseconds: 600,
      shutdownStrategy: { kind: "closeWindows" },
    },
    pathNeedsRepair: false,
  };
}

const crewChief = application(
  "application-crew-chief",
  "Crew Chief",
  "C:\\Program Files\\CrewChiefV4\\CrewChiefV4.exe",
);
const simHub = application(
  "application-simhub",
  "SimHub",
  "C:\\Program Files (x86)\\SimHub\\SimHubWPF.exe",
);
const virtualDesktopSwitcher = application(
  "application-desktop-switcher",
  "VirtualDesktopSwitcher",
  "C:\\Racing\\Tools\\VirtualDesktopSwitcher.exe",
);
const primarySim = {
  ...application(
    "application-lmu",
    "Le Mans Ultimate",
    "C:\\SteamLibrary\\steamapps\\common\\Le Mans Ultimate\\LeMansUltimate.exe",
  ),
  launchRecipe: {
    ...application(
      "application-lmu",
      "Le Mans Ultimate",
      "C:\\SteamLibrary\\steamapps\\common\\Le Mans Ultimate\\LeMansUltimate.exe",
    ).launchRecipe,
    source: {
      kind: "steam" as const,
      appId: 2_399_420,
      selector: null,
    },
    monitoredProcess: "LeMansUltimate.exe",
    monitoredExecutablePath: null,
  },
};

const orderedApplications: SessionApplicationSnapshot[] = [
  {
    applicationId: crewChief.id,
    name: crewChief.name,
    role: "supporting",
    requirement: "required",
    state: "pending",
  },
  {
    applicationId: simHub.id,
    name: simHub.name,
    role: "supporting",
    requirement: "optional",
    state: "pending",
  },
  {
    applicationId: virtualDesktopSwitcher.id,
    name: virtualDesktopSwitcher.name,
    role: "supporting",
    requirement: "optional",
    state: "pending",
  },
  {
    applicationId: primarySim.id,
    name: primarySim.name,
    role: "primarySim",
    requirement: null,
    state: "pending",
  },
];

function snapshotFor(
  previewState: string,
): Pick<AppSnapshot, "session" | "applicationProcesses"> {
  const applications = structuredClone(orderedApplications);
  const sessionState: SessionState =
    previewState === "starting"
      ? "starting"
      : previewState === "active"
        ? "active"
        : previewState === "closing"
          ? "closing"
          : "idle";

  if (previewState === "prestart") {
    return { session: idleSessionSnapshot(), applicationProcesses: [] };
  }
  if (previewState === "starting") {
    applications[0]!.state = "running";
    applications[1]!.state = "starting";
    return {
      session: {
        state: sessionState,
        activeProfileId: "profile-endurance",
        applications,
        summary: null,
      },
      applicationProcesses: [
        {
          applicationId: crewChief.id,
          status: "running",
          ownership: "sessionOwned",
          identity: {
            pid: 7_148,
            creationTime: "133822945071480000",
            canonicalExecutablePath:
              crewChief.launchRecipe.source.kind === "directExecutable"
                ? crewChief.launchRecipe.source.executablePath
                : "",
          },
          output: null,
        },
        {
          applicationId: simHub.id,
          status: "starting",
          ownership: "sessionOwned",
          identity: {
            pid: 7_660,
            creationTime: "133822945076600000",
            canonicalExecutablePath:
              simHub.launchRecipe.source.kind === "directExecutable"
                ? simHub.launchRecipe.source.executablePath
                : "",
          },
          output: null,
        },
      ],
    };
  }
  if (previewState === "failed") {
    applications[0]!.state = "failed";
    return {
      session: {
        state: "idle",
        activeProfileId: null,
        applications,
        summary: {
          profileId: "profile-endurance",
          events: [
            {
              applicationId: crewChief.id,
              name: crewChief.name,
              kind: "launchFailed",
            },
          ],
        },
      },
      applicationProcesses: [
        {
          applicationId: crewChief.id,
          status: "failed",
          ownership: null,
          identity: null,
          output: null,
        },
      ],
    };
  }

  for (const application of applications) {
    application.state = "running";
  }
  if (previewState === "closing") {
    applications[2]!.state = "detached";
    applications[3]!.state = "stopping";
  }
  return {
    session: {
      state: sessionState,
      activeProfileId: "profile-endurance",
      applications,
      summary: null,
    },
    applicationProcesses: orderedApplications.map((application, index) => ({
      applicationId: application.applicationId,
      status:
        previewState === "closing" && index === orderedApplications.length - 1
          ? "stopping"
          : previewState === "closing" && index === 2
            ? "runningPreExisting"
            : "running",
      ownership:
        previewState === "closing" && index === 2
          ? "preExisting"
          : "sessionOwned",
      identity: {
        pid: 8_000 + index * 512,
        creationTime: `133822945${8_000 + index * 512}00000`,
        canonicalExecutablePath: application.name,
      },
      output: null,
    })),
  };
}

export function createM4PreviewBridge(
  previewState: string,
): InMemoryNativeBridge {
  const sessionState = snapshotFor(previewState);
  const snapshot: AppSnapshot = {
    applicationName: "Formation Lap",
    foundationStatus: "ready",
    settings: {
      startWithWindows: false,
      theme: "system",
      reduceMotion: false,
      automaticUpdateChecks: true,
      updateChannel: "stable",
    },
    updates: {
      formationLap: { kind: "unknown", reason: "Not checked yet." },
      applications: [],
      lastAutomaticCheckUnixSeconds: null,
      resultDeferred: false,
    },
    profiles: [
      {
        id: "profile-endurance",
        name: "Le Mans evening",
        primarySimName: primarySim.name,
      },
    ],
    selectedProfile: {
      id: "profile-endurance",
      name: "Le Mans evening",
      primarySim,
      supportingApplications: [
        {
          application: crewChief,
          requirement: "required",
          keepRunning: false,
        },
        {
          application: simHub,
          requirement: "optional",
          keepRunning: false,
        },
        {
          application: virtualDesktopSwitcher,
          requirement: "optional",
          keepRunning: true,
        },
      ],
      vrEnabled: true,
      preferredVrLaunchMode: "openXr",
      closeSession: { stopSteamVr: true },
    },
    ...sessionState,
  };
  return new InMemoryNativeBridge(snapshot);
}
