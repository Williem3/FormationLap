import { render, screen } from "@testing-library/react";
import { App } from "../../app/App";
import { InMemoryNativeBridge } from "../../native-bridge/in-memory-native-bridge";
import { describe, expect, it } from "vitest";
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

describe("Diagnostics behavior", () => {
  it("shows a sanitized local diagnostic export with no upload path", async () => {
    const user = userEvent.setup();
    render(<App bridge={new InMemoryNativeBridge(lifecycleSnapshot())} />);

    await user.click(
      await screen.findByRole("button", { name: "Diagnostics" }),
    );

    const exportField = await screen.findByRole("textbox", {
      name: "Diagnostic export",
    });
    const exportValue = (exportField as HTMLTextAreaElement).value;
    expect(exportValue).toContain('"telemetryUpload": false');
    expect(exportValue).not.toContain("canonicalExecutablePath");
    expect(screen.getByText(/does not upload this export/)).toBeVisible();
  });
});
