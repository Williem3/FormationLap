import { render, screen, within } from "@testing-library/react";
import { App } from "../../app/App";
import { InMemoryNativeBridge } from "../../native-bridge/in-memory-native-bridge";
import { describe, expect, it, vi } from "vitest";
import userEvent from "@testing-library/user-event";
import type {
  ApplicationProcessSnapshot,
  AppSnapshot,
} from "../../generated/bindings";
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

describe("Dashboard behavior", () => {
  it("runs only the Primary Sim for Test Game Launch and shows a sanitized result", async () => {
    const user = userEvent.setup();
    const bridge = new InMemoryNativeBridge(lifecycleSnapshot());
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "Test game launch" }),
    );

    const result = await screen.findByRole("status", {
      name: "Test Game Launch result",
    });
    expect(within(result).getAllByText("healthy.exe")).toHaveLength(2);
    expect(result).not.toHaveTextContent(String.raw`C:\Fixtures`);
    expect(screen.getByRole("button", { name: "Start session" })).toBeEnabled();
  });
  it("remembers the Dashboard VR choice and locks it while a Session is active", async () => {
    const user = userEvent.setup();
    const bridge = new InMemoryNativeBridge(lifecycleSnapshot());
    const { unmount } = render(<App bridge={bridge} />);

    const vr = await screen.findByLabelText("VR");
    await user.click(vr);
    expect(vr).toBeChecked();
    expect((await bridge.getAppSnapshot()).selectedProfile?.vrEnabled).toBe(
      true,
    );

    unmount();
    const active = lifecycleSnapshot();
    active.session.state = "active";
    active.session.activeProfileId = "profile-lifecycle";
    render(<App bridge={new InMemoryNativeBridge(active)} />);
    expect(await screen.findByLabelText("VR")).toBeDisabled();
  });
  it("renders locally resolved executable icons in lifecycle rows", async () => {
    const snapshot = lifecycleSnapshot();
    const supportingApplication = {
      id: "simhub-lifecycle",
      name: "SimHub",
      launchRecipe: {
        source: {
          kind: "directExecutable" as const,
          executablePath: "C:\\Fixtures\\SimHub.exe",
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
    snapshot.selectedProfile!.supportingApplications = [
      {
        application: supportingApplication,
        requirement: "optional",
        keepRunning: false,
      },
    ];
    snapshot.applicationIcons = [
      {
        applicationId: supportingApplication.id,
        icon: {
          kind: "localData",
          media_type: "image/x-icon",
          data_base64: "AAABAA==",
        },
      },
      {
        applicationId: snapshot.selectedProfile!.primarySim.id,
        icon: {
          kind: "localData",
          media_type: "image/x-icon",
          data_base64: "AAACAA==",
        },
      },
    ];
    const bridge = new InMemoryNativeBridge(snapshot);
    const { container } = render(<App bridge={bridge} />);

    await screen.findAllByText("SimHub");
    expect(
      container.querySelector(
        '.application-icon img[src="data:image/x-icon;base64,AAABAA=="]',
      ),
    ).toBeTruthy();
    expect(
      container.querySelector(
        '.application-icon img[src="data:image/x-icon;base64,AAACAA=="]',
      ),
    ).toBeTruthy();
  });
  it("starts a Session through NativeBridge and renders its authoritative Formation Rail", async () => {
    const user = userEvent.setup();
    const bridge = new InMemoryNativeBridge(lifecycleSnapshot());
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "Start session" }),
    );

    expect(
      await screen.findByRole("button", { name: "Cancel startup" }),
    ).toBeEnabled();
    const rail = screen.getByRole("list", { name: "Formation Rail" });
    expect(within(rail).getByText("Healthy fixture")).toBeVisible();
    expect(within(rail).getByText("Starting")).toBeVisible();
  });
  it("locks the active profile and exposes Close Session without an unsolicited summary", async () => {
    const user = userEvent.setup();
    const snapshot = lifecycleSnapshot();
    snapshot.applicationProcesses = [processSnapshot()];
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
    render(<App bridge={new InMemoryNativeBridge(snapshot)} />);

    expect(
      await screen.findByRole("button", { name: "Close session" }),
    ).toBeEnabled();
    expect(screen.getByRole("button", { name: "Edit profile" })).toBeDisabled();
    expect(screen.queryByText("Session notes")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Close session" }));

    expect(
      await screen.findByRole("button", { name: "Closing session…" }),
    ).toBeDisabled();
  });
  it("uses the Session snapshot for Formation Rail state and reveals its summary only when Idle", async () => {
    const snapshot = lifecycleSnapshot();
    snapshot.applicationProcesses = [processSnapshot()];
    snapshot.session = {
      state: "idle",
      activeProfileId: null,
      applications: [
        {
          applicationId: "sim-lifecycle",
          name: "Healthy fixture",
          role: "primarySim",
          requirement: null,
          state: "failed",
        },
      ],
      summary: {
        profileId: "profile-lifecycle",
        events: [
          {
            applicationId: "sim-lifecycle",
            name: "Healthy fixture",
            kind: "launchFailed",
          },
        ],
      },
    };
    render(<App bridge={new InMemoryNativeBridge(snapshot)} />);

    const rail = await screen.findByRole("list", {
      name: "Formation Rail",
    });
    expect(within(rail).getByText("Failed")).toBeVisible();
    expect(within(rail).queryByText("Running")).not.toBeInTheDocument();
    expect(screen.getByText("Session notes")).toBeVisible();
    expect(screen.getByText("Did not finish startup")).toBeVisible();
  });
  it("gives Formation Rail stopped, pending, and running nodes their lifecycle tones", async () => {
    const snapshot = lifecycleSnapshot();
    const profile = snapshot.selectedProfile!;
    const stoppedApplication = {
      ...profile.primarySim,
      id: "stopped-lifecycle",
      name: "Stopped fixture",
    };
    const pendingApplication = {
      ...profile.primarySim,
      id: "pending-lifecycle",
      name: "Pending fixture",
    };
    const preExistingApplication = {
      ...profile.primarySim,
      id: "pre-existing-lifecycle",
      name: "Pre-existing fixture",
    };
    profile.supportingApplications = [
      stoppedApplication,
      pendingApplication,
      preExistingApplication,
    ].map((application) => ({
      application,
      requirement: "optional" as const,
      keepRunning: false,
    }));
    snapshot.session = {
      state: "starting",
      activeProfileId: profile.id,
      applications: [
        {
          applicationId: stoppedApplication.id,
          name: stoppedApplication.name,
          role: "supporting",
          requirement: "optional",
          state: "stopped",
        },
        {
          applicationId: pendingApplication.id,
          name: pendingApplication.name,
          role: "supporting",
          requirement: "optional",
          state: "pending",
        },
        {
          applicationId: preExistingApplication.id,
          name: preExistingApplication.name,
          role: "supporting",
          requirement: "optional",
          state: "runningPreExisting",
        },
        {
          applicationId: profile.primarySim.id,
          name: profile.primarySim.name,
          role: "primarySim",
          requirement: null,
          state: "running",
        },
      ],
      summary: null,
    };
    render(<App bridge={new InMemoryNativeBridge(snapshot)} />);

    const rail = await screen.findByRole("list", {
      name: "Formation Rail",
    });
    expect(
      within(rail).getByText("Stopped fixture").closest("li"),
    ).toHaveAttribute("data-rail-tone", "danger");
    expect(
      within(rail).getByText("Pending fixture").closest("li"),
    ).toHaveAttribute("data-rail-tone", "warm");
    expect(
      within(rail).getByText("Pre-existing fixture").closest("li"),
    ).toHaveAttribute("data-rail-tone", "running");
    expect(
      within(rail).getByText("Healthy fixture").closest("li"),
    ).toHaveAttribute("data-rail-tone", "running");
  });
  it("uses solid start lights instead of ordinal labels in the Formation Rail", async () => {
    const { container } = render(
      <App bridge={new InMemoryNativeBridge(lifecycleSnapshot())} />,
    );

    const rail = await screen.findByRole("list", {
      name: "Formation Rail",
    });
    expect(rail.querySelector(".race-light")).toBeTruthy();
    expect(container.querySelector(".rail-index")).toBeNull();
  });
  it("uses race-start copy and celebrates when every Formation Rail node is running", async () => {
    const initial = render(
      <App bridge={new InMemoryNativeBridge(lifecycleSnapshot())} />,
    );
    expect(
      await screen.findByRole("heading", {
        name: "Drivers Start Your Engines!",
      }),
    ).toBeVisible();
    expect(screen.queryByText("Startup sequence")).not.toBeInTheDocument();
    expect(
      screen.queryByRole("list", { name: "Startup sequence" }),
    ).not.toBeInTheDocument();
    initial.unmount();

    const ready = lifecycleSnapshot();
    ready.applicationProcesses = [processSnapshot({ status: "running" })];
    render(<App bridge={new InMemoryNativeBridge(ready)} />);
    expect(
      await screen.findByRole("heading", { name: "And Away we go!" }),
    ).toBeVisible();
  });
  it("starts one configured application and renders authoritative lifecycle state", async () => {
    const user = userEvent.setup();
    const bridge = new InMemoryNativeBridge(lifecycleSnapshot());
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "Start Healthy fixture" }),
    );

    expect(
      await screen.findByRole("status", {
        name: "Healthy fixture: Starting",
      }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Exit Healthy fixture" }),
    ).toBeEnabled();
    expect(
      screen.getByRole("button", { name: "Restart Healthy fixture" }),
    ).toBeEnabled();
  });
  it("requires explicit confirmation before exiting a Pre-existing Process", async () => {
    const user = userEvent.setup();
    const snapshot = lifecycleSnapshot();
    snapshot.applicationProcesses = [
      processSnapshot({
        status: "runningPreExisting",
        ownership: "preExisting",
      }),
    ];
    const bridge = new InMemoryNativeBridge(snapshot);
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "Exit Healthy fixture" }),
    );

    expect(
      await screen.findByRole("heading", {
        name: "Control a Pre-existing Process?",
      }),
    ).toBeVisible();
    expect(screen.getByText(/current Session does not own/)).toBeVisible();

    await user.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: "Exit Healthy fixture",
      }),
    );

    expect(
      await screen.findByRole("status", {
        name: "Healthy fixture: Stopped",
      }),
    ).toBeVisible();
  });
  it("warns about unsaved work before force termination", async () => {
    const user = userEvent.setup();
    const snapshot = lifecycleSnapshot();
    snapshot.applicationProcesses = [
      processSnapshot({
        status: "stopping",
      }),
    ];
    snapshot.pendingProcessConfirmation = {
      token: "native-confirmation-token",
      applicationId: "sim-lifecycle",
      action: "exit",
      identity: snapshot.applicationProcesses[0]!.identity!,
    };
    const bridge = new InMemoryNativeBridge(snapshot);
    render(<App bridge={bridge} />);

    expect(
      await screen.findByRole("heading", {
        name: "Force stop Healthy fixture?",
      }),
    ).toBeVisible();
    expect(screen.getByText(/may lose unsaved work/)).toBeVisible();

    await user.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: "Force stop Healthy fixture",
      }),
    );

    expect(
      await screen.findByRole("status", {
        name: "Healthy fixture: Stopped",
      }),
    ).toBeVisible();
  });
  it("cancels the native destructive intent when the force dialog is dismissed", async () => {
    const user = userEvent.setup();
    const snapshot = lifecycleSnapshot();
    snapshot.applicationProcesses = [
      processSnapshot({
        status: "stopping",
      }),
    ];
    snapshot.pendingProcessConfirmation = {
      token: "native-cancel-token",
      applicationId: "sim-lifecycle",
      action: "restart",
      identity: snapshot.applicationProcesses[0]!.identity!,
    };
    const bridge = new InMemoryNativeBridge(snapshot);
    render(<App bridge={bridge} />);

    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect((await bridge.getAppSnapshot()).pendingProcessConfirmation).toBe(
      undefined,
    );
  });
  it("keeps Force stop available for a Stopping Process while closing a Session", async () => {
    const snapshot = lifecycleSnapshot();
    snapshot.applicationProcesses = [processSnapshot({ status: "stopping" })];
    snapshot.session = {
      state: "closing",
      activeProfileId: "profile-lifecycle",
      applications: [
        {
          applicationId: "sim-lifecycle",
          name: "Healthy fixture",
          role: "primarySim",
          requirement: null,
          state: "stopping",
        },
      ],
      summary: null,
    };
    snapshot.pendingProcessConfirmation = {
      token: "native-close-token",
      applicationId: "sim-lifecycle",
      action: "sessionClose",
      identity: snapshot.applicationProcesses[0]!.identity!,
    };
    render(<App bridge={new InMemoryNativeBridge(snapshot)} />);

    expect(
      await screen.findByRole("heading", {
        name: "Force stop Healthy fixture?",
      }),
    ).toBeVisible();
  });
  it("shows bounded local console output and truncation state", async () => {
    const user = userEvent.setup();
    const snapshot = lifecycleSnapshot();
    snapshot.applicationProcesses = [
      processSnapshot({
        output: {
          stdout: "fixture ready\n",
          stderr: "diagnostic tail\n",
          truncated: true,
        },
      }),
    ];
    const bridge = new InMemoryNativeBridge(snapshot);
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "View Output" }),
    );

    expect(
      screen.getByRole("heading", { name: "Healthy fixture output" }),
    ).toBeVisible();
    expect(screen.getByText(/fixture ready/)).toHaveTextContent(
      "diagnostic tail",
    );
    expect(screen.getByText(/Earlier output was discarded/)).toBeVisible();
  });
  it("reserves a disabled No Output action when an application has no captured output", async () => {
    render(<App bridge={new InMemoryNativeBridge(lifecycleSnapshot())} />);

    expect(
      await screen.findByRole("button", { name: "No Output" }),
    ).toBeDisabled();
  });
  it("shows the native elevated-launch recovery message on the Dashboard", async () => {
    const user = userEvent.setup();
    const bridge = new InMemoryNativeBridge(lifecycleSnapshot());
    Object.assign(bridge, {
      startSession: vi.fn().mockRejectedValue({
        message:
          "The elevated helper could not launch Virtual Desktop Switcher.",
        recovery: "Approve the Windows prompt and verify the application path.",
      }),
    });
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "Start session" }),
    );

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "The elevated helper could not launch Virtual Desktop Switcher. Approve the Windows prompt and verify the application path.",
    );
  });
  it("refreshes the authoritative starting snapshot when the first launch fails", async () => {
    const user = userEvent.setup();
    const bridge = new InMemoryNativeBridge(lifecycleSnapshot());
    const startSession = bridge.startSession.bind(bridge);
    Object.assign(bridge, {
      startSession: vi.fn(async (payload) => {
        await startSession(payload);
        throw new Error("The elevated helper rejected the launch.");
      }),
    });
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "Start session" }),
    );

    expect(
      await screen.findByRole("button", { name: "Cancel startup" }),
    ).toBeEnabled();
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "The elevated helper rejected the launch.",
    );
  });
  it("explains when an application exits during startup", async () => {
    const snapshot = lifecycleSnapshot();
    snapshot.applicationProcesses = [
      processSnapshot({ status: "failed", identity: null }),
    ];
    render(<App bridge={new InMemoryNativeBridge(snapshot)} />);

    expect(
      await screen.findByText(
        "Healthy fixture exited during startup. Check its executable path and enter each launch argument on a separate line.",
      ),
    ).toBeVisible();
  });
});
