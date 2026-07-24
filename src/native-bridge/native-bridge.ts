import type { AppSnapshot } from "../generated/bindings";

export interface NativeBridge {
  getAppSnapshot(): Promise<AppSnapshot>;
}
