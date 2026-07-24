import type { AppSnapshot, ProfileApplication } from "../generated/bindings";
import { InMemoryNativeBridge } from "../native-bridge/in-memory-native-bridge";
import { createM3PreviewSnapshot } from "./m3-preview";

function configureApplication(
  application: ProfileApplication,
  id: string,
  name: string,
  executablePath: string,
) {
  application.id = id;
  application.name = name;
  application.launchRecipe.source = {
    kind: "directExecutable",
    executablePath,
  };
}

function createM9PreviewSnapshot(): AppSnapshot {
  const snapshot = createM3PreviewSnapshot();
  const profile = snapshot.selectedProfile;
  if (!profile) {
    throw new Error("M9 preview requires a selected Racing Profile");
  }

  const [lmuffb, tradingPaints, simHub] = profile.supportingApplications.map(
    ({ application }) => application,
  );
  if (!lmuffb || !tradingPaints || !simHub) {
    throw new Error("M9 preview requires three Supporting Applications");
  }
  configureApplication(
    lmuffb,
    "application-lmuffb",
    "LMUFFB",
    "C:\\Program Files\\LMUFFB\\LMUFFB.exe",
  );
  configureApplication(
    tradingPaints,
    "application-trading-paints",
    "Trading Paints",
    "C:\\Program Files\\Trading Paints\\Trading Paints.exe",
  );
  configureApplication(
    simHub,
    "application-simhub",
    "SimHub",
    "C:\\Program Files (x86)\\SimHub\\SimHubWPF.exe",
  );

  snapshot.applicationProcesses = [];
  snapshot.settings.updateChannel = "beta";
  snapshot.updates = {
    formationLap: {
      kind: "updateAvailable",
      currentVersion: "0.9.0",
      latestVersion: "1.0.0-beta.2",
    },
    applications: [
      {
        applicationId: lmuffb.id,
        name: lmuffb.name,
        status: { kind: "current", currentVersion: "2.1.0" },
        informationUrl: "https://github.com/coasting-nc/LMUFFB/releases/latest",
      },
      {
        applicationId: tradingPaints.id,
        name: tradingPaints.name,
        status: {
          kind: "updateAvailable",
          currentVersion: "2.0.36",
          latestVersion: "2.0.37",
        },
        informationUrl: null,
      },
      {
        applicationId: simHub.id,
        name: simHub.name,
        status: {
          kind: "unknown",
          reason: "The official SimHub page changed or was ambiguous.",
        },
        informationUrl: null,
      },
    ],
    lastAutomaticCheckUnixSeconds: 1_774_368_000,
    resultDeferred: false,
  };
  return snapshot;
}

export function createM9PreviewBridge(): InMemoryNativeBridge {
  return new InMemoryNativeBridge(createM9PreviewSnapshot());
}
