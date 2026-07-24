import type {
  AppSnapshot,
  DiscoverySnapshot,
  SupportingApplicationRecommendation,
} from "../generated/bindings";
import { InMemoryNativeBridge } from "../native-bridge/in-memory-native-bridge";
import { idleSessionSnapshot } from "../session/session-snapshot";

const emptySnapshot: AppSnapshot = {
  applicationName: "Formation Lap",
  foundationStatus: "ready",
  session: idleSessionSnapshot(),
  applicationProcesses: [],
  profiles: [],
  selectedProfile: null,
};

const discovery: DiscoverySnapshot = {
  primarySims: [
    {
      id: "le-mans-ultimate",
      name: "Le Mans Ultimate",
      steamAppId: 2_399_420,
    },
    {
      id: "iracing",
      name: "iRacing",
      steamAppId: 2_668_100,
    },
  ],
  supportingApplications: [
    { id: "lmuffb", name: "LMUFFB" },
    { id: "simhub", name: "SimHub" },
  ],
  installedPrimarySims: [
    {
      id: "le-mans-ultimate",
      name: "Le Mans Ultimate",
      installation: {
        kind: "steam",
        appId: 2_399_420,
        install_directory:
          "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Le Mans Ultimate",
      },
      icon: { kind: "generic" },
    },
    {
      id: "iracing",
      name: "iRacing",
      installation: {
        kind: "directExecutable",
        executablePath: "C:\\Program Files (x86)\\iRacing\\iRacingUI.exe",
      },
      icon: { kind: "generic" },
    },
  ],
  installedSupportingApplications: [
    {
      id: "lmuffb",
      name: "LMUFFB",
      installation: {
        kind: "directExecutable",
        executablePath: "C:\\Racing\\Tools\\LMUFFB\\LMUFFB.exe",
      },
      icon: { kind: "generic" },
    },
    {
      id: "simhub",
      name: "SimHub",
      installation: {
        kind: "directExecutable",
        executablePath: "C:\\Program Files (x86)\\SimHub\\SimHubWPF.exe",
      },
      icon: { kind: "generic" },
    },
  ],
};

const recommendations: SupportingApplicationRecommendation[] = [
  {
    id: "lmuffb",
    name: "LMUFFB",
    rank: "recommended",
    updateProvider: {
      kind: "githubReleases",
      repository: "coasting-nc/LMUFFB",
    },
  },
  {
    id: "simhub",
    name: "SimHub",
    rank: "compatible",
    updateProvider: null,
  },
];

export function createM5PreviewBridge(): InMemoryNativeBridge {
  return new InMemoryNativeBridge(emptySnapshot, discovery, {
    "le-mans-ultimate": recommendations,
  });
}
