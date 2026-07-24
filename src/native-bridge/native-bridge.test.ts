import type { AppSnapshot } from "../generated/bindings";
import { InMemoryNativeBridge } from "./in-memory-native-bridge";
import { describe, expect, it } from "vitest";

describe("InMemoryNativeBridge", () => {
  it("returns the same authoritative snapshot shape as the native adapter", async () => {
    const snapshot: AppSnapshot = {
      applicationName: "Formation Lap",
      foundationStatus: "ready",
    };
    const bridge = new InMemoryNativeBridge(snapshot);

    await expect(bridge.getAppSnapshot()).resolves.toEqual(snapshot);
  });
});
