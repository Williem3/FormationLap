import type { AppSnapshot } from "../generated/bindings";
import type { NativeBridge } from "./native-bridge";

export class InMemoryNativeBridge implements NativeBridge {
  readonly #snapshot: AppSnapshot;

  constructor(snapshot: AppSnapshot) {
    this.#snapshot = snapshot;
  }

  getAppSnapshot(): Promise<AppSnapshot> {
    return Promise.resolve(structuredClone(this.#snapshot));
  }
}
