import type { AppSnapshot } from "../generated/bindings";
import { InMemoryNativeBridge } from "../native-bridge/in-memory-native-bridge";

type M2Preview = "m2-wizard" | "m2-editor";

const editorSnapshot: AppSnapshot = {
  applicationName: "Formation Lap",
  foundationStatus: "ready",
  applicationProcesses: [],
  profiles: [
    {
      id: "profile-endurance",
      name: "Le Mans evening",
      primarySimName: "Le Mans Ultimate",
    },
  ],
  selectedProfile: {
    id: "profile-endurance",
    name: "Le Mans evening",
    primarySim: {
      id: "application-lmu",
      name: "Le Mans Ultimate",
      launchRecipe: {
        source: { kind: "steam", appId: 2399420 },
        arguments: ["-windowed"],
        workingDirectory: null,
        monitoredProcess: "LeMansUltimate.exe",
        consoleVisibility: "hidden",
        elevated: false,
        startupTimeoutSeconds: 45,
        postStartDelayMilliseconds: 0,
        shutdownStrategy: { kind: "closeWindows" },
      },
      pathNeedsRepair: false,
    },
    supportingApplications: [
      {
        application: {
          id: "application-crew-chief",
          name: "Crew Chief",
          launchRecipe: {
            source: {
              kind: "directExecutable",
              executablePath: "C:\\Program Files\\CrewChiefV4\\CrewChiefV4.exe",
            },
            arguments: [],
            workingDirectory: "C:\\Program Files\\CrewChiefV4",
            monitoredProcess: null,
            consoleVisibility: "hidden",
            elevated: false,
            startupTimeoutSeconds: 30,
            postStartDelayMilliseconds: 1000,
            shutdownStrategy: { kind: "closeWindows" },
          },
          pathNeedsRepair: false,
        },
        requirement: "required",
        keepRunning: false,
      },
      {
        application: {
          id: "application-simhub",
          name: "SimHub",
          launchRecipe: {
            source: {
              kind: "directExecutable",
              executablePath: "C:\\Program Files (x86)\\SimHub\\SimHubWPF.exe",
            },
            arguments: [],
            workingDirectory: "C:\\Program Files (x86)\\SimHub",
            monitoredProcess: null,
            consoleVisibility: "hidden",
            elevated: false,
            startupTimeoutSeconds: 30,
            postStartDelayMilliseconds: 1500,
            shutdownStrategy: { kind: "closeWindows" },
          },
          pathNeedsRepair: false,
        },
        requirement: "optional",
        keepRunning: true,
      },
    ],
    vrEnabled: true,
    preferredVrLaunchMode: "openXr",
    closeSession: { stopSteamVr: true },
  },
};

export function createM2PreviewBridge(
  preview: M2Preview,
): InMemoryNativeBridge {
  if (preview === "m2-wizard") {
    return new InMemoryNativeBridge({
      applicationName: "Formation Lap",
      foundationStatus: "ready",
      applicationProcesses: [],
      profiles: [],
      selectedProfile: null,
    });
  }

  return new InMemoryNativeBridge(editorSnapshot);
}
