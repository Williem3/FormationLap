import type { AppSnapshot, ThemePreference } from "../generated/bindings";
import { InMemoryNativeBridge } from "../native-bridge/in-memory-native-bridge";
import { idleSessionSnapshot } from "../session/session-snapshot";

export function createM8PreviewBridge(theme: ThemePreference) {
  const snapshot: AppSnapshot = {
    applicationName: "Formation Lap",
    foundationStatus: "ready",
    settings: {
      startWithWindows: false,
      theme,
      reduceMotion: false,
    },
    profiles: [
      {
        id: "profile-endurance",
        name: "Endurance",
        primarySimName: "Le Mans Ultimate",
      },
    ],
    selectedProfile: null,
    applicationProcesses: [],
    session: idleSessionSnapshot(),
  };
  return new InMemoryNativeBridge(snapshot);
}
