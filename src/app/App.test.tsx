import { fireEvent, render, screen } from "@testing-library/react";
import { App } from "./App";
import { InMemoryNativeBridge } from "../native-bridge/in-memory-native-bridge";
import { describe, expect, it } from "vitest";
import userEvent from "@testing-library/user-event";

describe("Formation Lap shell", () => {
  it("renders the native snapshot through NativeBridge", async () => {
    const bridge = new InMemoryNativeBridge({
      applicationName: "Formation Lap",
      foundationStatus: "ready",
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
      profiles: [],
      selectedProfile: null,
    });
    render(<App bridge={bridge} />);

    await user.click(
      await screen.findByRole("button", { name: "New profile" }),
    );
    await user.type(screen.getByLabelText("Profile name"), "Le Mans evening");
    await user.type(
      screen.getByLabelText("Primary Sim name"),
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
