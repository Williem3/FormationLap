import { getAppSnapshot } from "../generated/bindings";
import type { NativeBridge } from "./native-bridge";

export class TauriNativeBridge implements NativeBridge {
  getAppSnapshot = getAppSnapshot;
}
