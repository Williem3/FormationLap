import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { App } from "./App";
import { InMemoryNativeBridge } from "../native-bridge/in-memory-native-bridge";
import { describe, expect, it, vi } from "vitest";
import userEvent from "@testing-library/user-event";
import type {
  ApplicationProcessSnapshot,
  AppSnapshot,
  DiscoverySnapshot,
  SupportingApplicationRecommendation,
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
      screen.getByRole("heading", {
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
    const bridge = new InMemoryNativeBridge(snapshot);
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", {
        name: "Force stop Healthy fixture",
      }),
    );

    expect(
      screen.getByRole("heading", {
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

  it("keeps Force stop available for a Stopping Process while closing a Session", async () => {
    const user = userEvent.setup();
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
    render(<App bridge={new InMemoryNativeBridge(snapshot)} />);

    const forceStop = await screen.findByRole("button", {
      name: "Force stop Healthy fixture",
    });
    expect(forceStop).toBeEnabled();
    await user.click(forceStop);
    expect(
      screen.getByRole("heading", { name: "Force stop Healthy fixture?" }),
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

  it("does not present unavailable Session actions as enabled", async () => {
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

    render(<App bridge={bridge} />);

    expect(
      await screen.findByRole("button", { name: "Start session" }),
    ).toBeDisabled();
  });

  it("creates the first Racing Profile through NativeBridge", async () => {
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
      profiles: [],
      selectedProfile: null,
    });
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "New profile" }),
    );
    await user.type(screen.getByLabelText("Profile name"), "Le Mans evening");
    await user.type(
      await screen.findByLabelText("Primary Sim name"),
      "Le Mans Ultimate",
    );
    await user.click(
      screen.getByRole("button", { name: "Create Racing Profile" }),
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

  it("offers discovered curated sims, ranked recommendations, and Manual Entry", async () => {
    const user = userEvent.setup();
    const discovery: DiscoverySnapshot = {
      primarySims: [
        {
          id: "le-mans-ultimate",
          name: "Le Mans Ultimate",
          steamAppId: 2399420,
        },
        {
          id: "assetto-corsa",
          name: "Assetto Corsa",
          steamAppId: 244210,
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
            appId: 2399420,
            install_directory: String.raw`C:\Steam\Le Mans Ultimate`,
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
            executablePath: String.raw`C:\Tools\LMUFFB\LMUFFB.exe`,
          },
          icon: { kind: "generic" },
        },
        {
          id: "simhub",
          name: "SimHub",
          installation: {
            kind: "directExecutable",
            executablePath: String.raw`C:\Program Files (x86)\SimHub\SimHubWPF.exe`,
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
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "New profile" }),
    );
    expect(
      await screen.findByRole("heading", { name: "Choose a Primary Sim" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", {
        name: "Use Le Mans Ultimate (Steam)",
      }),
    ).toBeVisible();
    expect(screen.queryByText("Assetto Corsa")).not.toBeInTheDocument();

    await user.click(
      screen.getByRole("button", {
        name: "Use Le Mans Ultimate (Steam)",
      }),
    );
    const recommendationRegion = await screen.findByRole("region", {
      name: "Recommended for Le Mans Ultimate",
    });
    expect(
      within(recommendationRegion)
        .getAllByRole("checkbox")
        .map((checkbox) => checkbox.getAttribute("aria-label")),
    ).toEqual(["Add LMUFFB", "Add SimHub"]);
    expect(within(recommendationRegion).getByText("Recommended")).toBeVisible();
    expect(within(recommendationRegion).getByText("Compatible")).toBeVisible();

    await user.click(
      screen.getByRole("button", { name: "Enter a sim manually" }),
    );
    expect(await screen.findByLabelText("Primary Sim name")).toBeVisible();
    expect(screen.getByLabelText("Primary Sim source")).toHaveValue("direct");
    expect(await screen.findByLabelText("Executable path")).toBeVisible();
  });

  it("creates a profile with selected discovered Supporting Applications", async () => {
    const user = userEvent.setup();
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
      {
        primarySims: [
          {
            id: "le-mans-ultimate",
            name: "Le Mans Ultimate",
            steamAppId: 2399420,
          },
        ],
        supportingApplications: [{ id: "lmuffb", name: "LMUFFB" }],
        installedPrimarySims: [
          {
            id: "le-mans-ultimate",
            name: "Le Mans Ultimate",
            installation: {
              kind: "steam",
              appId: 2399420,
              install_directory: String.raw`C:\Steam\Le Mans Ultimate`,
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
              executablePath: String.raw`C:\Tools\LMUFFB\LMUFFB.exe`,
            },
            icon: { kind: "generic" },
          },
        ],
      },
      {
        "le-mans-ultimate": [
          {
            id: "lmuffb",
            name: "LMUFFB",
            rank: "recommended",
            updateProvider: {
              kind: "githubReleases",
              repository: "coasting-nc/LMUFFB",
            },
          },
        ],
      },
    );
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "New profile" }),
    );
    await user.type(screen.getByLabelText("Profile name"), "LMU race");
    await user.click(
      await screen.findByRole("button", {
        name: "Use Le Mans Ultimate (Steam)",
      }),
    );
    await user.click(await screen.findByLabelText("Add LMUFFB"));
    await user.click(
      screen.getByRole("button", { name: "Create Racing Profile" }),
    );

    expect(
      await screen.findByRole("heading", { level: 1, name: "LMU race" }),
    ).toBeVisible();
    const savedProfile = (await bridge.getAppSnapshot()).selectedProfile;
    expect(savedProfile?.primarySim.launchRecipe.source).toEqual({
      kind: "steam",
      appId: 2399420,
      selector: null,
    });
    expect(savedProfile?.supportingApplications).toMatchObject([
      {
        application: {
          name: "LMUFFB",
          launchRecipe: {
            source: {
              kind: "directExecutable",
              executablePath: String.raw`C:\Tools\LMUFFB\LMUFFB.exe`,
            },
            workingDirectory: String.raw`C:\Tools\LMUFFB`,
          },
          pathNeedsRepair: false,
        },
        requirement: "optional",
        keepRunning: false,
      },
    ]);
  });

  it("edits the selected Racing Profile through NativeBridge", async () => {
    const user = userEvent.setup();
    const profile = {
      id: "profile-1",
      name: "Endurance",
      primarySim: {
        id: "sim-1",
        name: "Le Mans Ultimate",
        launchRecipe: {
          source: {
            kind: "directExecutable" as const,
            executablePath: "",
          },
          arguments: [],
          workingDirectory: null,
          monitoredProcess: null,
          monitoredExecutablePath: null,
          consoleVisibility: "hidden" as const,
          elevated: false,
          startupTimeoutSeconds: 30,
          postStartDelayMilliseconds: 0,
          shutdownStrategy: { kind: "closeWindows" as const },
        },
        pathNeedsRepair: true,
      },
      supportingApplications: [],
      vrEnabled: false,
      preferredVrLaunchMode: null,
      closeSession: { stopSteamVr: false },
    };
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
          id: profile.id,
          name: profile.name,
          primarySimName: profile.primarySim.name,
        },
      ],
      selectedProfile: profile,
    });
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "Edit profile" }),
    );
    const name = screen.getByLabelText("Profile name");
    await user.clear(name);
    await user.type(name, "Sunday endurance");
    await user.click(screen.getByLabelText("VR enabled by default"));
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    expect(
      await screen.findByRole("heading", {
        level: 1,
        name: "Sunday endurance",
      }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: /Sunday endurance/ }),
    ).toHaveAttribute("aria-current", "page");
    expect(screen.getByLabelText("VR")).toBeChecked();
  });

  it("keeps launch arguments, advanced startup settings, and shutdown policy compact", async () => {
    const user = userEvent.setup();
    const snapshot = lifecycleSnapshot();
    snapshot.selectedProfile!.supportingApplications = [
      {
        application: {
          id: "simhub-lifecycle",
          name: "SimHub",
          launchRecipe: {
            source: {
              kind: "directExecutable",
              executablePath: "C:\\Fixtures\\SimHub.exe",
            },
            arguments: [],
            workingDirectory: "C:\\Fixtures",
            monitoredProcess: "SimHub.exe",
            monitoredExecutablePath: "C:\\Fixtures\\SimHub.exe",
            consoleVisibility: "visible",
            elevated: false,
            startupTimeoutSeconds: 30,
            postStartDelayMilliseconds: 0,
            shutdownStrategy: { kind: "closeWindows" },
          },
          pathNeedsRepair: false,
        },
        requirement: "optional",
        keepRunning: false,
      },
    ];
    const bridge = new InMemoryNativeBridge(snapshot);
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "Edit profile" }),
    );
    await user.click(screen.getAllByText("Launch Recipe details")[0]!);

    expect(screen.getAllByText("No arguments.")).toHaveLength(2);
    await user.click(
      screen.getByRole("button", { name: "Add Primary Sim argument" }),
    );
    await user.type(screen.getByLabelText("Primary Sim arguments 1"), "-novr");

    await user.click(screen.getAllByText("Advanced startup settings")[0]!);
    const hiddenConsole = screen.getAllByRole("checkbox", {
      name: "Hide console",
    })[0]!;
    expect(hiddenConsole).toBeChecked();
    await user.click(hiddenConsole);

    const requirement = screen.getByLabelText("Requirement for SimHub");
    const policy = requirement.closest(".supporting-policy");
    if (!(policy instanceof HTMLElement)) {
      throw new Error("Supporting Application policy should be grouped");
    }
    expect(within(policy).getByLabelText("Shutdown strategy")).toHaveValue(
      "closeWindows",
    );
    await user.selectOptions(
      within(policy).getByLabelText("Shutdown strategy"),
      "consoleInterrupt",
    );
    const keepRunning = screen.getByRole("checkbox", {
      name: /Keep SimHub running/,
    });
    expect(keepRunning.closest("label")).toHaveClass("compact-check");

    await user.click(screen.getByRole("button", { name: "Save changes" }));

    const savedProfile = (await bridge.getAppSnapshot()).selectedProfile;
    expect(savedProfile?.primarySim.launchRecipe.arguments).toEqual(["-novr"]);
    expect(savedProfile?.primarySim.launchRecipe.consoleVisibility).toBe(
      "visible",
    );
    expect(
      savedProfile?.supportingApplications[0]?.application.launchRecipe
        .shutdownStrategy,
    ).toEqual({ kind: "consoleInterrupt" });
  });

  it("keeps one Supporting Application editor open while retaining its unsaved draft", async () => {
    const user = userEvent.setup();
    const snapshot = lifecycleSnapshot();
    snapshot.selectedProfile!.supportingApplications = [
      {
        application: {
          id: "simhub-accordion",
          name: "SimHub",
          launchRecipe: {
            source: {
              kind: "directExecutable",
              executablePath: "C:\\Fixtures\\SimHub.exe",
            },
            arguments: [],
            workingDirectory: "C:\\Fixtures",
            monitoredProcess: "SimHub.exe",
            monitoredExecutablePath: "C:\\Fixtures\\SimHub.exe",
            consoleVisibility: "hidden",
            elevated: false,
            startupTimeoutSeconds: 30,
            postStartDelayMilliseconds: 0,
            shutdownStrategy: { kind: "closeWindows" },
          },
          pathNeedsRepair: false,
        },
        requirement: "optional",
        keepRunning: false,
      },
      {
        application: {
          id: "crew-chief-accordion",
          name: "Crew Chief",
          launchRecipe: {
            source: {
              kind: "directExecutable",
              executablePath: "C:\\Fixtures\\CrewChief.exe",
            },
            arguments: [],
            workingDirectory: "C:\\Fixtures",
            monitoredProcess: "CrewChief.exe",
            monitoredExecutablePath: "C:\\Fixtures\\CrewChief.exe",
            consoleVisibility: "hidden",
            elevated: false,
            startupTimeoutSeconds: 30,
            postStartDelayMilliseconds: 0,
            shutdownStrategy: { kind: "closeWindows" },
          },
          pathNeedsRepair: false,
        },
        requirement: "required",
        keepRunning: false,
      },
    ];
    const bridge = new InMemoryNativeBridge(snapshot);
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "Edit profile" }),
    );

    const simHubToggle = screen.getByRole("button", {
      name: /SimHub.*Supporting Application editor/,
    });
    const crewChiefToggle = screen.getByRole("button", {
      name: /Crew Chief.*Supporting Application editor/,
    });
    const crewChiefEditor = crewChiefToggle.closest(".supporting-editor-row");
    if (!(crewChiefEditor instanceof HTMLElement)) {
      throw new Error("Crew Chief should be contained in its editor row");
    }
    expect(simHubToggle).toHaveAttribute("aria-expanded", "true");
    expect(crewChiefToggle).toHaveAttribute("aria-expanded", "false");
    expect(
      within(crewChiefEditor).queryByText("Launch Recipe details"),
    ).not.toBeInTheDocument();

    await user.clear(screen.getByLabelText("Supporting Application 1 name"));
    await user.type(
      screen.getByLabelText("Supporting Application 1 name"),
      "SimHub draft",
    );
    await user.click(crewChiefToggle);

    expect(simHubToggle).toHaveAttribute("aria-expanded", "false");
    expect(crewChiefToggle).toHaveAttribute("aria-expanded", "true");
    expect(
      screen.queryByLabelText("Supporting Application 1 name"),
    ).not.toBeInTheDocument();
    expect(
      within(crewChiefEditor).getByText("Launch Recipe details"),
    ).toBeVisible();
    expect(
      within(crewChiefEditor)
        .getByText("Launch Recipe details")
        .closest("details"),
    ).not.toHaveAttribute("open");

    await user.click(simHubToggle);
    expect(screen.getByLabelText("Supporting Application 1 name")).toHaveValue(
      "SimHub draft",
    );

    await user.click(screen.getByRole("button", { name: "Save changes" }));
    const savedProfile = (await bridge.getAppSnapshot()).selectedProfile;
    expect(savedProfile?.supportingApplications[0]?.application.name).toBe(
      "SimHub draft",
    );
  });

  it("shows profile save failures beside the editor save action", async () => {
    const user = userEvent.setup();
    const bridge = new InMemoryNativeBridge(lifecycleSnapshot());
    vi.spyOn(bridge, "saveProfile").mockRejectedValueOnce(new Error("Denied"));
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "Edit profile" }),
    );
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    const error = await screen.findByRole("alert");
    expect(error).toHaveTextContent("The Racing Profile could not be saved");
    expect(error.closest(".editor-header")).not.toBeNull();
  });

  it("names each executable that blocks imported-profile approval", async () => {
    const user = userEvent.setup();
    const snapshot = lifecycleSnapshot();
    snapshot.selectedProfile!.primarySim.pathNeedsRepair = true;
    snapshot.profiles[0]!.reviewStatus = "needsReview";
    const bridge = new InMemoryNativeBridge(snapshot);
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "Edit profile" }),
    );

    expect(
      screen.getByText("Healthy fixture needs an executable path."),
    ).toBeVisible();
  });

  it("selects an executable through the native file picker instead of requiring a typed path", async () => {
    const user = userEvent.setup();
    const bridge = new InMemoryNativeBridge(lifecycleSnapshot());
    const pickExecutablePath = vi
      .fn()
      .mockResolvedValue(String.raw`C:\Racing\iRacing\iRacingSim64DX11.exe`);
    Object.assign(bridge, { pickExecutablePath });
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "Edit profile" }),
    );
    await user.click(screen.getByText("Launch Recipe details"));
    await user.click(
      screen.getByRole("button", {
        name: "Browse for Primary Sim executable",
      }),
    );

    expect(pickExecutablePath).toHaveBeenCalledWith(
      String.raw`C:\Fixtures\healthy.exe`,
    );
    expect(
      screen.getByRole("button", { name: "Browse for Primary Sim executable" }),
    ).toHaveClass("path-browse-button");
    expect(screen.getByLabelText("Primary Sim executable path")).toHaveValue(
      String.raw`C:\Racing\iRacing\iRacingSim64DX11.exe`,
    );
    expect(
      screen.queryByLabelText("Primary Sim working directory"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByLabelText("Primary Sim monitored process"),
    ).not.toBeInTheDocument();
    expect(
      screen.getByText(/monitor this exact executable automatically/),
    ).toBeVisible();
  });

  it("omits the Windows extended-length prefix from displayed executable paths", async () => {
    const user = userEvent.setup();
    const snapshot = lifecycleSnapshot();
    const selectedProfile = snapshot.selectedProfile;
    if (!selectedProfile) {
      throw new Error("fixture profile should be selected");
    }
    selectedProfile.primarySim.launchRecipe.source = {
      kind: "directExecutable",
      executablePath: String.raw`\\?\C:\Fixtures\healthy.exe`,
    };
    const bridge = new InMemoryNativeBridge(snapshot);
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "Edit profile" }),
    );
    await user.click(screen.getByText("Launch Recipe details"));

    expect(screen.getByLabelText("Primary Sim executable path")).toHaveValue(
      String.raw`C:\Fixtures\healthy.exe`,
    );
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

  it("duplicates the selected Racing Profile through NativeBridge", async () => {
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
          name: "Endurance",
          primarySimName: "Le Mans Ultimate",
        },
      ],
      selectedProfile: {
        id: "profile-1",
        name: "Endurance",
        primarySim: {
          id: "sim-1",
          name: "Le Mans Ultimate",
          launchRecipe: {
            source: { kind: "steam", appId: 2399420, selector: null },
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

    const duplicateAction = await screen.findByRole("button", {
      name: "Duplicate profile",
    });
    duplicateAction.focus();
    await user.keyboard("{Enter}");
    expect(screen.getByLabelText("Duplicate name")).toHaveFocus();

    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(duplicateAction).toHaveFocus();

    await user.keyboard("{Enter}");
    const name = screen.getByLabelText("Duplicate name");
    await user.clear(name);
    await user.type(name, "Endurance test");
    await user.click(screen.getByRole("button", { name: "Create duplicate" }));

    expect(
      await screen.findByRole("button", { name: /Endurance test/ }),
    ).toBeVisible();
    expect(
      screen.getByRole("heading", { level: 1, name: "Endurance" }),
    ).toBeVisible();
  });

  it("requires confirmation before deleting the selected Racing Profile", async () => {
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
          name: "Endurance",
          primarySimName: "Le Mans Ultimate",
        },
      ],
      selectedProfile: {
        id: "profile-1",
        name: "Endurance",
        primarySim: {
          id: "sim-1",
          name: "Le Mans Ultimate",
          launchRecipe: {
            source: { kind: "steam", appId: 2399420, selector: null },
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
      await screen.findByRole("button", { name: "Delete profile" }),
    );

    expect(
      screen.getByRole("heading", { name: "Delete Endurance?" }),
    ).toBeVisible();
    expect(
      screen.getByRole("heading", { level: 1, name: "Endurance" }),
    ).toBeVisible();

    await user.click(screen.getByRole("button", { name: "Delete Endurance" }));

    expect(
      await screen.findByRole("heading", {
        level: 2,
        name: "Secure foundation ready",
      }),
    ).toBeVisible();
    expect(
      screen.queryByRole("button", { name: /Endurance/ }),
    ).not.toBeInTheDocument();
  });

  it("exports and imports portable Racing Profiles through NativeBridge", async () => {
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
          name: "Endurance",
          primarySimName: "Le Mans Ultimate",
        },
      ],
      selectedProfile: {
        id: "profile-1",
        name: "Endurance",
        primarySim: {
          id: "sim-1",
          name: "Le Mans Ultimate",
          launchRecipe: {
            source: { kind: "steam", appId: 2399420, selector: null },
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
      await screen.findByRole("button", { name: "Export profile" }),
    );
    expect(
      screen.getByRole("heading", { name: "Export Endurance" }),
    ).toBeVisible();
    expect(
      (screen.getByLabelText("Portable profile JSON") as HTMLTextAreaElement)
        .value,
    ).toContain('"name": "Endurance"');
    await user.click(screen.getByRole("button", { name: "Close export" }));

    await user.click(screen.getByRole("button", { name: "Import profile" }));
    fireEvent.change(screen.getByLabelText("Portable profile JSON"), {
      target: {
        value: JSON.stringify({
          schemaVersion: 1,
          name: "Imported sprint",
          primarySim: {
            name: "Automobilista 2",
            launchRecipe: {
              source: { kind: "steam", appId: 1066890, selector: null },
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
          },
          supportingApplications: [],
          vrEnabled: false,
          preferredVrLaunchMode: null,
          closeSession: { stopSteamVr: false },
        }),
      },
    });
    await user.click(
      screen.getByRole("button", { name: "Import Racing Profile" }),
    );

    expect(
      await screen.findByRole("button", { name: /Imported sprint/ }),
    ).toBeVisible();
  });

  it("returns to the Dashboard after approving an elevated imported profile", async () => {
    const user = userEvent.setup();
    const bridge = new InMemoryNativeBridge(lifecycleSnapshot());
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "Import profile" }),
    );
    fireEvent.change(screen.getByLabelText("Portable profile JSON"), {
      target: {
        value: JSON.stringify({
          schemaVersion: 1,
          name: "Imported review",
          primarySim: {
            name: "Automobilista 2",
            launchRecipe: {
              source: { kind: "steam", appId: 1066890, selector: null },
              arguments: ["-novr"],
              workingDirectory: null,
              monitoredProcess: null,
              monitoredExecutablePath: null,
              consoleVisibility: "hidden",
              elevated: true,
              startupTimeoutSeconds: 30,
              postStartDelayMilliseconds: 0,
              shutdownStrategy: { kind: "closeWindows" },
            },
          },
          supportingApplications: [],
          vrEnabled: false,
          preferredVrLaunchMode: null,
          closeSession: { stopSteamVr: false },
        }),
      },
    });
    await user.click(
      screen.getByRole("button", { name: "Import Racing Profile" }),
    );
    await user.click(
      await screen.findByRole("button", { name: /Imported review/ }),
    );

    expect(
      screen.getByRole("heading", {
        name: "Review imported executable settings",
      }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Start session" }),
    ).toBeDisabled();

    await user.click(
      screen.getByRole("button", { name: "Review profile configuration" }),
    );
    await user.click(
      screen.getByLabelText(
        "I reviewed executable paths, arguments, working directories, elevation, monitored executables, and stop recipes.",
      ),
    );
    await user.click(
      screen.getByLabelText(/Approve privileged recipe for Automobilista 2/),
    );
    await user.click(
      screen.getByRole("button", { name: "Save and approve profile" }),
    );

    expect(
      await screen.findByRole("button", { name: "Start session" }),
    ).toBeEnabled();
    expect(
      screen.queryByRole("heading", {
        name: "Review imported executable settings",
      }),
    ).not.toBeInTheDocument();
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

  it("renders Formation Lap and Supporting Application update advice without a third-party install action", async () => {
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

    render(<App bridge={new InMemoryNativeBridge(snapshot)} />);

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
      screen.getByText(/Checks resume when the Session is idle/),
    ).toBeVisible();
  });

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
