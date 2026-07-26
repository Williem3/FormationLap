import type { AppSnapshot } from "../generated/bindings";

export type SnapshotState =
  | { kind: "loading" }
  | { kind: "ready"; snapshot: AppSnapshot }
  | { kind: "error" };
