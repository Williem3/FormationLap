import { render, screen } from "@testing-library/react";
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
});
