import type { AppSnapshot, ProcessIdentity } from "../generated/bindings";
import { InMemoryNativeBridge } from "../native-bridge/in-memory-native-bridge";
import { idleSessionSnapshot } from "../session/session-snapshot";

function identity(pid: number, executablePath: string): ProcessIdentity {
  return {
    pid,
    creationTime: `133822944${pid}00000`,
    canonicalExecutablePath: executablePath,
  };
}

const virtualDesktopSwitcher = {
  id: "application-virtual-desktop-switcher",
  name: "VirtualDesktopSwitcher",
  launchRecipe: {
    source: {
      kind: "directExecutable" as const,
      executablePath: "C:\\Racing\\Tools\\VirtualDesktopSwitcher.exe",
    },
    arguments: ["--monitor", "primary sim"],
    workingDirectory: "C:\\Racing\\Tools",
    monitoredProcess: null,
    consoleVisibility: "hidden" as const,
    elevated: false,
    startupTimeoutSeconds: 30,
    postStartDelayMilliseconds: 0,
    shutdownStrategy: { kind: "consoleInterrupt" as const },
  },
  pathNeedsRepair: false,
};

const crewChief = {
  id: "application-crew-chief",
  name: "Crew Chief",
  launchRecipe: {
    source: {
      kind: "directExecutable" as const,
      executablePath: "C:\\Program Files\\CrewChiefV4\\CrewChiefV4.exe",
    },
    arguments: [],
    workingDirectory: "C:\\Program Files\\CrewChiefV4",
    monitoredProcess: null,
    consoleVisibility: "hidden" as const,
    elevated: false,
    startupTimeoutSeconds: 30,
    postStartDelayMilliseconds: 1_000,
    shutdownStrategy: { kind: "closeWindows" as const },
  },
  pathNeedsRepair: false,
};

const simHub = {
  id: "application-simhub",
  name: "SimHub",
  launchRecipe: {
    source: {
      kind: "directExecutable" as const,
      executablePath: "C:\\Program Files (x86)\\SimHub\\SimHubWPF.exe",
    },
    arguments: [],
    workingDirectory: "C:\\Program Files (x86)\\SimHub",
    monitoredProcess: null,
    consoleVisibility: "hidden" as const,
    elevated: false,
    startupTimeoutSeconds: 30,
    postStartDelayMilliseconds: 1_500,
    shutdownStrategy: { kind: "closeWindows" as const },
  },
  pathNeedsRepair: false,
};

const leMansUltimate = {
  id: "application-lmu",
  name: "Le Mans Ultimate",
  launchRecipe: {
    source: { kind: "steam" as const, appId: 2_399_420 },
    arguments: ["-windowed"],
    workingDirectory: null,
    monitoredProcess: "LeMansUltimate.exe",
    consoleVisibility: "hidden" as const,
    elevated: false,
    startupTimeoutSeconds: 45,
    postStartDelayMilliseconds: 0,
    shutdownStrategy: { kind: "closeWindows" as const },
  },
  pathNeedsRepair: false,
};

const dashboardSnapshot: AppSnapshot = {
  applicationName: "Formation Lap",
  foundationStatus: "ready",
  session: idleSessionSnapshot(),
  applicationProcesses: [
    {
      applicationId: virtualDesktopSwitcher.id,
      status: "running",
      ownership: "sessionOwned",
      identity: identity(
        7_148,
        virtualDesktopSwitcher.launchRecipe.source.executablePath,
      ),
      output: {
        stdout: "Watching desktop 2 for Primary Sim focus.\n",
        stderr: "",
        truncated: false,
      },
    },
    {
      applicationId: crewChief.id,
      status: "runningPreExisting",
      ownership: "preExisting",
      identity: identity(7_660, crewChief.launchRecipe.source.executablePath),
      output: null,
    },
    {
      applicationId: simHub.id,
      status: "notResponding",
      ownership: "sessionOwned",
      identity: identity(8_172, simHub.launchRecipe.source.executablePath),
      output: null,
    },
    {
      applicationId: leMansUltimate.id,
      status: "failed",
      ownership: null,
      identity: null,
      output: null,
    },
  ],
  profiles: [
    {
      id: "profile-endurance",
      name: "Le Mans evening",
      primarySimName: leMansUltimate.name,
    },
  ],
  selectedProfile: {
    id: "profile-endurance",
    name: "Le Mans evening",
    primarySim: leMansUltimate,
    supportingApplications: [
      {
        application: virtualDesktopSwitcher,
        requirement: "required",
        keepRunning: false,
      },
      {
        application: crewChief,
        requirement: "optional",
        keepRunning: false,
      },
      {
        application: simHub,
        requirement: "optional",
        keepRunning: true,
      },
    ],
    vrEnabled: true,
    preferredVrLaunchMode: "openXr",
    closeSession: { stopSteamVr: true },
  },
};

export function createM3PreviewBridge(): InMemoryNativeBridge {
  return new InMemoryNativeBridge(dashboardSnapshot);
}
