import type {
  AppSnapshot,
  DiscoverySnapshot,
  SupportingApplicationRecommendation,
} from "../generated/bindings";
import { InMemoryNativeBridge } from "./in-memory-native-bridge";
import { idleSessionSnapshot } from "../session/session-snapshot";
import { describe, expect, it } from "vitest";

describe("InMemoryNativeBridge", () => {
  it("returns the same authoritative snapshot shape as the native adapter", async () => {
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
      session: idleSessionSnapshot(),
      applicationProcesses: [],
      profiles: [],
      selectedProfile: null,
    };
    const bridge = new InMemoryNativeBridge(snapshot);

    await expect(bridge.getAppSnapshot()).resolves.toEqual(snapshot);
  });

  it("supports the same typed profile creation behavior as the native adapter", async () => {
    const bridge = new InMemoryNativeBridge({
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
      session: idleSessionSnapshot(),
      applicationProcesses: [],
      profiles: [],
      selectedProfile: null,
    });

    const snapshot = await bridge.createProfile({
      name: "Le Mans evening",
      primarySimName: "Le Mans Ultimate",
    });

    expect(snapshot.profiles).toEqual([
      expect.objectContaining({
        name: "Le Mans evening",
        primarySimName: "Le Mans Ultimate",
      }),
    ]);
    expect(snapshot.selectedProfile?.name).toBe("Le Mans evening");
  });

  it("selects a newly created profile over the existing selection", async () => {
    const bridge = new InMemoryNativeBridge({
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
      session: idleSessionSnapshot(),
      applicationProcesses: [],
      profiles: [],
      selectedProfile: null,
    });
    await bridge.createProfile({
      name: "Le Mans Ultimate",
      primarySimName: "Le Mans Ultimate",
    });

    const snapshot = await bridge.createProfile({
      name: "iRacing",
      primarySimName: "iRacing",
    });

    expect(snapshot.selectedProfile?.name).toBe("iRacing");
  });

  it("returns the same discovery and recommendation contracts as the native adapter", async () => {
    const discovery: DiscoverySnapshot = {
      primarySims: [
        {
          id: "le-mans-ultimate",
          name: "Le Mans Ultimate",
          steamAppId: 2399420,
        },
      ],
      supportingApplications: [{ id: "lmuffb", name: "LMUFFB" }],
      installedPrimarySims: [],
      installedSupportingApplications: [],
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
    ];
    const bridge = new InMemoryNativeBridge(
      {
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
        session: idleSessionSnapshot(),
        applicationProcesses: [],
        profiles: [],
        selectedProfile: null,
      },
      discovery,
      { "le-mans-ultimate": recommendations },
    );

    await expect(bridge.discoverApplications()).resolves.toEqual(discovery);
    await expect(
      bridge.recommendApplications({ primarySimId: "le-mans-ultimate" }),
    ).resolves.toEqual(recommendations);
  });
});
