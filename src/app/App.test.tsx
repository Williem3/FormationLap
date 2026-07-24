import { fireEvent, render, screen, within } from "@testing-library/react";
import { App } from "./App";
import { InMemoryNativeBridge } from "../native-bridge/in-memory-native-bridge";
import { describe, expect, it } from "vitest";
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
    const rail = screen.getByRole("list", { name: "Startup sequence" });
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
      name: "Startup sequence",
    });
    expect(within(rail).getByText("Failed")).toBeVisible();
    expect(within(rail).queryByText("Running")).not.toBeInTheDocument();
    expect(screen.getByText("Session notes")).toBeVisible();
    expect(screen.getByText("Did not finish startup")).toBeVisible();
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
      await screen.findByRole("button", { name: "View output" }),
    );

    expect(
      screen.getByRole("heading", { name: "Healthy fixture output" }),
    ).toBeVisible();
    expect(screen.getByText(/fixture ready/)).toHaveTextContent(
      "diagnostic tail",
    );
    expect(screen.getByText(/Earlier output was discarded/)).toBeVisible();
  });

  it("renders the native snapshot through NativeBridge", async () => {
    const bridge = new InMemoryNativeBridge({
      applicationName: "Formation Lap",
      foundationStatus: "ready",
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
  });

  it("does not present unavailable Session actions as enabled", async () => {
    const bridge = new InMemoryNativeBridge({
      applicationName: "Formation Lap",
      foundationStatus: "ready",
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

  it("selects another Racing Profile from the sidebar", async () => {
    const user = userEvent.setup();
    const bridge = new InMemoryNativeBridge({
      applicationName: "Formation Lap",
      foundationStatus: "ready",
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
            source: { kind: "steam", appId: 244210 },
            arguments: [],
            workingDirectory: null,
            monitoredProcess: null,
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
            source: { kind: "steam", appId: 2399420 },
            arguments: [],
            workingDirectory: null,
            monitoredProcess: null,
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
            source: { kind: "steam", appId: 2399420 },
            arguments: [],
            workingDirectory: null,
            monitoredProcess: null,
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
            source: { kind: "steam", appId: 2399420 },
            arguments: [],
            workingDirectory: null,
            monitoredProcess: null,
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
              source: { kind: "steam", appId: 1066890 },
              arguments: [],
              workingDirectory: null,
              monitoredProcess: null,
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
});
