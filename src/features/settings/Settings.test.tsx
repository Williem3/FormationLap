import { render, screen, waitFor } from "@testing-library/react";
import { App } from "../../app/App";
import { InMemoryNativeBridge } from "../../native-bridge/in-memory-native-bridge";
import { describe, expect, it, vi } from "vitest";
import userEvent from "@testing-library/user-event";
import type { AppSnapshot } from "../../generated/bindings";
import { idleSessionSnapshot } from "../../session/session-snapshot";

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

describe("Settings behavior", () => {
  it("configures automatic checks and the signed Stable or Beta channel", async () => {
    const user = userEvent.setup();
    const bridge = new InMemoryNativeBridge(lifecycleSnapshot());
    render(<App bridge={bridge} />);

    await user.click(await screen.findByRole("button", { name: "Settings" }));
    await user.click(screen.getByLabelText("Automatic daily checks"));
    await user.click(screen.getByRole("button", { name: "Beta" }));

    await waitFor(async () => {
      const settings = (await bridge.getAppSnapshot()).settings;
      expect(settings.automaticUpdateChecks).toBe(false);
      expect(settings.updateChannel).toBe("beta");
    });
    expect(screen.getByRole("button", { name: "Check now" })).toBeEnabled();
    expect(screen.getByText(/verified first-party updates/)).toBeVisible();
    expect(
      screen.getByText(/GitHub Releases, Winget, and SimHub/),
    ).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Dashboard" }));
    expect(screen.getByText("Local data · Online checks off")).toBeVisible();
  });
  it("explains why the Formation Lap update state is Unknown", async () => {
    const user = userEvent.setup();
    const snapshot = lifecycleSnapshot();
    snapshot.updates.formationLap = {
      kind: "unknown",
      reason: "The official updater public key is not configured.",
    };
    render(<App bridge={new InMemoryNativeBridge(snapshot)} />);

    await user.click(await screen.findByRole("button", { name: "Settings" }));

    expect(screen.getByText("Unknown")).toBeVisible();
    expect(
      screen.getByText("The official updater public key is not configured."),
    ).toBeVisible();
  });
  it("renders Formation Lap and Supporting Application update advice without a third-party install action", async () => {
    const user = userEvent.setup();
    const snapshot = lifecycleSnapshot();
    const supportingApplication = {
      ...snapshot.selectedProfile!.primarySim,
      id: "lmuffb-profile-id",
      name: "LMUFFB",
    };
    snapshot.selectedProfile!.supportingApplications = [
      {
        application: supportingApplication,
        requirement: "optional",
        keepRunning: false,
      },
    ];
    snapshot.updates = {
      formationLap: {
        kind: "updateAvailable",
        currentVersion: "0.1.0",
        latestVersion: "1.0.0",
      },
      applications: [
        {
          applicationId: "lmuffb-profile-id",
          name: "LMUFFB",
          status: {
            kind: "updateAvailable",
            currentVersion: "1.4.0",
            latestVersion: "1.5.0",
          },
          informationUrl:
            "https://github.com/coasting-nc/LMUFFB/releases/tag/v1.5.0",
        },
      ],
      lastAutomaticCheckUnixSeconds: 1_000_000,
      resultDeferred: false,
    };

    const bridge = new InMemoryNativeBridge(snapshot);
    Object.assign(bridge, {
      installFormationLapUpdate: vi.fn().mockRejectedValue({
        message:
          "The verified update installer could not start (ShellExecute error 32).",
        recovery: "Run a fresh update check or install an official release.",
      }),
    });
    render(<App bridge={bridge} />);

    expect(
      await screen.findByText("Formation Lap 1.0.0 is available"),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Install verified update" }),
    ).toBeVisible();
    expect(screen.getByText("Update available · 1.5.0")).toBeVisible();
    expect(
      screen.queryByRole("button", { name: /Install LMUFFB/ }),
    ).not.toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "Install verified update" }),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "The verified update installer could not start (ShellExecute error 32). Run a fresh update check or install an official release.",
    );
  });
  it("keeps a manual update install action in Settings", async () => {
    const user = userEvent.setup();
    const snapshot = lifecycleSnapshot();
    snapshot.settings.automaticUpdateChecks = false;
    snapshot.updates.formationLap = {
      kind: "updateAvailable",
      currentVersion: "0.9.0-preview.2",
      latestVersion: "0.9.0-preview.4",
    };
    const bridge = new InMemoryNativeBridge(snapshot);
    const installFormationLapUpdate = vi.spyOn(
      bridge,
      "installFormationLapUpdate",
    );
    render(<App bridge={bridge} />);

    expect(
      screen.queryByText("Formation Lap 0.9.0-preview.4 is available"),
    ).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Settings" }));
    const install = screen.getByRole("button", {
      name: "Install update 0.9.0-preview.4",
    });
    expect(install).toBeEnabled();
    await user.click(install);
    expect(installFormationLapUpdate).toHaveBeenCalledOnce();
  });
  it("makes race-safe update deferral visible while a Session is active", async () => {
    const user = userEvent.setup();
    const snapshot = lifecycleSnapshot();
    snapshot.session.state = "active";
    snapshot.session.activeProfileId = snapshot.selectedProfile!.id;
    snapshot.updates.resultDeferred = true;
    render(<App bridge={new InMemoryNativeBridge(snapshot)} />);

    expect(
      await screen.findByText(/Update advice will appear after this Session/),
    ).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Settings" }));
    expect(screen.getByRole("button", { name: "Check now" })).toBeDisabled();
    expect(
      screen.getByText(
        /Checks and installation resume when the Session is idle/,
      ),
    ).toBeVisible();
  });
});
