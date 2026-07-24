import type { SessionSnapshot } from "../generated/bindings";

export function idleSessionSnapshot(): SessionSnapshot {
  return {
    state: "idle",
    activeProfileId: null,
    applications: [],
    summary: null,
  };
}
