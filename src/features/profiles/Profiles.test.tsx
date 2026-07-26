import { fireEvent, render, screen, within } from "@testing-library/react";
import { App } from "../../app/App";
import { InMemoryNativeBridge } from "../../native-bridge/in-memory-native-bridge";
import { describe, expect, it, vi } from "vitest";
import userEvent from "@testing-library/user-event";
import type {
  AppSnapshot,
  DiscoverySnapshot,
  SupportingApplicationRecommendation,
} from "../../generated/bindings";
import { idleSessionSnapshot } from "../../session/session-snapshot";
import {
  createAppSnapshot,
  createProfileApplication,
  createRacingProfile,
} from "../../test/app-snapshot-builder";

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

describe("Racing Profile behavior", () => {
  it("uses locally resolved icons in the profile sidebar and startup-order editor", async () => {
    const user = userEvent.setup();
    const primarySim = createProfileApplication({
      id: "primary-sim-icon",
      name: "Le Mans Ultimate",
    });
    const supportingApplication = createProfileApplication({
      id: "supporting-application-icon",
      name: "LMUFFB",
    });
    const profile = createRacingProfile({
      id: "profile-icons",
      name: "Le Mans evening",
      primarySim,
      supportingApplications: [
        {
          application: supportingApplication,
          requirement: "required",
          keepRunning: false,
        },
      ],
    });
    const bridge = new InMemoryNativeBridge(
      createAppSnapshot({
        profiles: [
          {
            id: profile.id,
            name: profile.name,
            primarySimName: primarySim.name,
            primarySimApplicationId: primarySim.id,
          },
        ],
        selectedProfile: profile,
        applicationIcons: [
          {
            applicationId: primarySim.id,
            icon: {
              kind: "localData",
              media_type: "image/x-icon",
              data_base64: "AAABAA==",
            },
          },
          {
            applicationId: supportingApplication.id,
            icon: {
              kind: "localData",
              media_type: "image/x-icon",
              data_base64: "AAACAA==",
            },
          },
        ],
      }),
    );
    render(<App bridge={bridge} />);

    expect(
      await screen.findByRole("button", { name: /Le Mans evening/ }),
    ).toBeVisible();
    expect(
      document.querySelector(
        '.profile-nav-icon img[src="data:image/x-icon;base64,AAABAA=="]',
      ),
    ).not.toBeNull();

    await user.click(screen.getByRole("button", { name: "Edit profile" }));

    expect(
      document.querySelector(
        '.supporting-application-icon img[src="data:image/x-icon;base64,AAACAA=="]',
      ),
    ).not.toBeNull();
    expect(
      document.querySelector(
        '.locked-game-row img[src="data:image/x-icon;base64,AAABAA=="]',
      ),
    ).not.toBeNull();
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
    const bridge = new InMemoryNativeBridge(createAppSnapshot());
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
  it("creates and configures a new Racing Profile without changing the previous profile", async () => {
    const user = userEvent.setup();
    const initialSnapshot = lifecycleSnapshot();
    const previousProfile = structuredClone(initialSnapshot.selectedProfile);
    const previousProfileId = previousProfile!.id;
    const bridge = new InMemoryNativeBridge(initialSnapshot);
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "New profile" }),
    );
    await user.type(screen.getByLabelText("Profile name"), "iRacing sprint");
    await user.type(
      await screen.findByLabelText("Primary Sim name"),
      "iRacing",
    );
    await user.click(
      screen.getByRole("button", { name: "Create Racing Profile" }),
    );

    expect(
      await screen.findByRole("heading", {
        level: 1,
        name: "iRacing sprint",
      }),
    ).toBeVisible();
    const createdSnapshot = await bridge.getAppSnapshot();
    expect(createdSnapshot.selectedProfile).toEqual(
      expect.objectContaining({
        name: "iRacing sprint",
        primarySim: expect.objectContaining({
          name: "iRacing",
          pathNeedsRepair: true,
        }),
      }),
    );

    const previousSnapshot = await bridge.selectProfile({
      profileId: previousProfileId,
    });
    expect(previousSnapshot.selectedProfile).toEqual(previousProfile);
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
        { id: "go-fast", name: "Go Fast" },
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
          icon: {
            kind: "localData",
            media_type: "image/x-icon",
            data_base64: "AAABAA==",
          },
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
          profileDefaults: {
            arguments: [],
            consoleVisibility: "hidden",
            elevated: false,
            startupTimeoutSeconds: 30,
            postStartDelayMilliseconds: 0,
            shutdownStrategy: { kind: "closeWindows" },
            requirement: "optional",
            keepRunning: false,
          },
          icon: {
            kind: "localData",
            media_type: "image/x-icon",
            data_base64: "AAACAA==",
          },
        },
        {
          id: "simhub",
          name: "SimHub",
          installation: {
            kind: "directExecutable",
            executablePath: String.raw`C:\Program Files (x86)\SimHub\SimHubWPF.exe`,
          },
          profileDefaults: {
            arguments: [],
            consoleVisibility: "hidden",
            elevated: false,
            startupTimeoutSeconds: 30,
            postStartDelayMilliseconds: 0,
            shutdownStrategy: { kind: "closeWindows" },
            requirement: "optional",
            keepRunning: false,
          },
          icon: { kind: "generic" },
        },
        {
          id: "go-fast",
          name: "Go Fast",
          installation: {
            kind: "directExecutable",
            executablePath: String.raw`D:\Racing\GoFast\GoFast.exe`,
          },
          profileDefaults: {
            arguments: [],
            consoleVisibility: "hidden",
            elevated: false,
            startupTimeoutSeconds: 30,
            postStartDelayMilliseconds: 0,
            shutdownStrategy: { kind: "closeWindows" },
            requirement: "optional",
            keepRunning: false,
          },
          icon: {
            kind: "localData",
            media_type: "image/x-icon",
            data_base64: "AAABAA==",
          },
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
    await user.click(screen.getByLabelText("Add LMUFFB"));
    expect(
      document.querySelector(
        '.game-order-icon img[src="data:image/x-icon;base64,AAABAA=="]',
      ),
    ).not.toBeNull();
    expect(
      document.querySelector(
        '.order-application-row img[src="data:image/x-icon;base64,AAACAA=="]',
      ),
    ).not.toBeNull();
    const recommendationRegion = await screen.findByRole("region", {
      name: "Recommended for Le Mans Ultimate",
    });
    expect(
      within(recommendationRegion)
        .getAllByRole("checkbox")
        .map((checkbox) => checkbox.getAttribute("aria-label")),
    ).toEqual(["Add LMUFFB", "Add SimHub", "Add Go Fast"]);
    expect(within(recommendationRegion).getByText("Recommended")).toBeVisible();
    expect(within(recommendationRegion).getByText("Compatible")).toBeVisible();
    expect(
      within(recommendationRegion).getByText("Other detected applications"),
    ).toBeVisible();
    expect(screen.getByLabelText("Add Go Fast")).toBeVisible();
    expect(
      recommendationRegion.querySelector(
        'img[src="data:image/x-icon;base64,AAABAA=="]',
      ),
    ).not.toBeNull();

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
            profileDefaults: {
              arguments: ["--profile=LMU"],
              consoleVisibility: "visible",
              elevated: false,
              startupTimeoutSeconds: 45,
              postStartDelayMilliseconds: 500,
              shutdownStrategy: { kind: "closeWindows" },
              requirement: "required",
              keepRunning: true,
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
            arguments: ["--profile=LMU"],
            consoleVisibility: "visible",
            startupTimeoutSeconds: 45,
            postStartDelayMilliseconds: 500,
            workingDirectory: String.raw`C:\Tools\LMUFFB`,
          },
          pathNeedsRepair: false,
        },
        requirement: "required",
        keepRunning: true,
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
  it("limits preferred VR launch modes to OpenVR / SteamVR and Oculus", async () => {
    const user = userEvent.setup();
    render(<App bridge={new InMemoryNativeBridge(lifecycleSnapshot())} />);

    await user.click(
      await screen.findByRole("button", { name: "Edit profile" }),
    );

    const mode = screen.getByLabelText("Preferred VR Launch Mode");
    expect(
      within(mode)
        .getAllByRole("option")
        .map((option) => option.textContent),
    ).toEqual(["OpenVR / SteamVR", "Oculus"]);
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

    await user.click(
      screen.getAllByText("Shutdown and advanced launch settings")[0]!,
    );
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
  it("groups Primary Sim Steam and runtime details behind compact disclosures", async () => {
    const user = userEvent.setup();
    const snapshot = lifecycleSnapshot();
    snapshot.selectedProfile!.primarySim.launchRecipe.source = {
      kind: "steam",
      appId: 2399420,
      selector: null,
    };
    snapshot.selectedProfile!.primarySim.launchRecipe.monitoredProcess =
      "Le Mans Ultimate.exe";
    snapshot.selectedProfile!.primarySim.launchRecipe.monitoredExecutablePath =
      "B:\\SteamLibrary\\steamapps\\common\\Le Mans Ultimate\\Le Mans Ultimate.exe";
    const bridge = new InMemoryNativeBridge(snapshot);
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "Edit profile" }),
    );
    await user.click(screen.getByText("Launch Recipe details"));

    const steamSettings = screen.getByText("Steam launch settings");
    const steamDetails = steamSettings.closest("details");
    const processMatching = screen.getByText("Process matching");
    const processDetails = processMatching.closest("details");
    const runtimeSettings = screen.getByText(
      "Shutdown and advanced launch settings",
    );
    const runtimeDetails = runtimeSettings.closest("details");
    if (
      !(steamDetails instanceof HTMLDetailsElement) ||
      !(processDetails instanceof HTMLDetailsElement) ||
      !(runtimeDetails instanceof HTMLDetailsElement)
    ) {
      throw new Error("Launch Recipe groups should use native disclosures");
    }
    expect(steamDetails).not.toHaveAttribute("open");
    expect(processDetails).not.toHaveAttribute("open");
    expect(runtimeDetails).not.toHaveAttribute("open");

    await user.click(steamSettings);
    expect(
      screen.getByLabelText("Primary Sim Steam launch option"),
    ).toBeVisible();

    await user.click(processMatching);
    expect(screen.getByLabelText("Primary Sim monitored process")).toHaveValue(
      "Le Mans Ultimate.exe",
    );

    await user.click(runtimeSettings);
    expect(screen.getByLabelText("Shutdown strategy")).toHaveValue(
      "closeWindows",
    );
    expect(
      screen.getByRole("checkbox", { name: "Hide console" }),
    ).toBeChecked();
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
});
