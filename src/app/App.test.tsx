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
});
