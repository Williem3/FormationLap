import { render, screen, waitFor } from "@testing-library/react";
import { App } from "./App";
import { InMemoryNativeBridge } from "../native-bridge/in-memory-native-bridge";
import { describe, expect, it } from "vitest";
import userEvent from "@testing-library/user-event";
import type {
  ApplicationProcessSnapshot,
  AppSnapshot,
} from "../generated/bindings";
import { idleSessionSnapshot } from "../session/session-snapshot";

function lifecycleSnapshot(): AppSnapshot {
  const primarySim = {
    id: "sim-lifecycle",
    name: "Healthy fixture",
    launchRecipe: {
      source: {
        kind: "directExecutable" as const,
        executablePath: "C:\\Fixtures\\healthy.exe",
      },
      arguments: [],
      workingDirectory: "C:\\Fixtures",
      monitoredProcess: null,
      monitoredExecutablePath: null,
      consoleVisibility: "hidden" as const,
      elevated: false,
      startupTimeoutSeconds: 3,
      postStartDelayMilliseconds: 0,
      shutdownStrategy: { kind: "closeWindows" as const },
    },
    pathNeedsRepair: false,
  };
  return {
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
    profiles: [
      {
        id: "profile-lifecycle",
        name: "Fixture profile",
        primarySimName: primarySim.name,
      },
    ],
    selectedProfile: {
      id: "profile-lifecycle",
      name: "Fixture profile",
      primarySim,
      supportingApplications: [],
      vrEnabled: false,
      preferredVrLaunchMode: null,
      closeSession: { stopSteamVr: false },
    },
  };
}

function processSnapshot(
  overrides: Partial<ApplicationProcessSnapshot> = {},
): ApplicationProcessSnapshot {
  return {
    applicationId: "sim-lifecycle",
    status: "running",
    ownership: "sessionOwned",
    identity: {
      pid: 4242,
      creationTime: "133822233344455566",
      canonicalExecutablePath: "C:\\Fixtures\\healthy.exe",
    },
    output: null,
    ...overrides,
  };
}

describe("Formation Lap shell", () => {
  it("renders the native snapshot through NativeBridge", async () => {
    const bridge = new InMemoryNativeBridge({
      applicationName: "Formation Lap",
      foundationStatus: "ready",
      settings: {
        startWithWindows: false,
        theme: "system",
        reduceMotion: false,
        automaticUpdateChecks: false,
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

    render(<App bridge={bridge} />);

    expect(
      await screen.findByRole("heading", {
        level: 1,
        name: "Formation Lap",
      }),
    ).toBeVisible();
    expect(
      screen.getByRole("heading", {
        level: 2,
        name: "Secure foundation ready",
      }),
    ).toBeVisible();
    expect(screen.getByText("Local data · Online checks off")).toBeVisible();
    expect(
      screen.getByText(/GitHub Releases, Winget, and SimHub/),
    ).toBeVisible();
  });
  it("selects another Racing Profile from the sidebar", async () => {
    const user = userEvent.setup();
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
      profiles: [
        {
          id: "profile-1",
          name: "Assetto Corsa",
          primarySimName: "Assetto Corsa",
        },
        {
          id: "profile-2",
          name: "Le Mans evening",
          primarySimName: "Le Mans Ultimate",
        },
      ],
      selectedProfile: {
        id: "profile-1",
        name: "Assetto Corsa",
        primarySim: {
          id: "sim-1",
          name: "Assetto Corsa",
          launchRecipe: {
            source: { kind: "steam", appId: 244210, selector: null },
            arguments: [],
            workingDirectory: null,
            monitoredProcess: null,
            monitoredExecutablePath: null,
            consoleVisibility: "hidden",
            elevated: false,
            startupTimeoutSeconds: 30,
            postStartDelayMilliseconds: 0,
            shutdownStrategy: { kind: "closeWindows" },
          },
          pathNeedsRepair: false,
        },
        supportingApplications: [],
        vrEnabled: false,
        preferredVrLaunchMode: null,
        closeSession: { stopSteamVr: false },
      },
    });
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: /Le Mans evening/ }),
    );

    expect(
      await screen.findByRole("heading", {
        level: 1,
        name: "Le Mans evening",
      }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: /Le Mans evening/ }),
    ).toHaveAttribute("aria-current", "page");
  });
  it("persists desktop settings and applies theme and reduced-motion preferences", async () => {
    const user = userEvent.setup();
    const bridge = new InMemoryNativeBridge(lifecycleSnapshot());
    render(<App bridge={bridge} />);

    await user.click(await screen.findByRole("button", { name: "Settings" }));
    expect(
      screen.getByRole("heading", { level: 1, name: "Settings" }),
    ).toBeVisible();

    await user.click(screen.getByLabelText(/Start with Windows/));
    await user.click(screen.getByRole("button", { name: "Dark" }));
    await user.click(screen.getByLabelText(/Reduce motion/));

    await waitFor(async () => {
      const settings = (await bridge.getAppSnapshot()).settings;
      expect(settings).toEqual({
        startWithWindows: true,
        theme: "dark",
        reduceMotion: true,
        automaticUpdateChecks: true,
        updateChannel: "stable",
      });
    });
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(document.documentElement.dataset.reduceMotion).toBe("true");
    expect(screen.getByText(/Racing Profiles never auto-start/)).toBeVisible();
  });
  it("makes active-Session Quit choices explicit and honors leave-running", async () => {
    const user = userEvent.setup();
    const snapshot = lifecycleSnapshot();
    snapshot.session = {
      state: "active",
      activeProfileId: "profile-lifecycle",
      applications: [
        {
          applicationId: "sim-lifecycle",
          name: "Healthy fixture",
          role: "primarySim",
          requirement: null,
          state: "running",
        },
      ],
      summary: null,
    };
    snapshot.applicationProcesses = [processSnapshot()];
    const bridge = new InMemoryNativeBridge(snapshot);
    render(<App bridge={bridge} />);

    await user.click(await screen.findByRole("button", { name: "Quit…" }));
    expect(
      screen.getByRole("heading", {
        name: "What should happen to this Session?",
      }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Close Session and quit" }),
    ).toBeVisible();

    await user.click(
      screen.getByRole("button", { name: "Leave applications running" }),
    );
    const detached = await bridge.getAppSnapshot();
    expect(detached.session.state).toBe("idle");
    expect(detached.applicationProcesses[0]?.ownership).toBe("preExisting");
    expect(detached.applicationProcesses[0]?.status).toBe("runningPreExisting");
  });
});
