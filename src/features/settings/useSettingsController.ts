import { useState } from "react";
import type { AppSnapshot, DesktopSettings } from "../../generated/bindings";
import type { NativeBridge } from "../../native-bridge/native-bridge";
import { commandErrorMessage } from "../../ui/presentation";

interface SettingsControllerOptions {
  bridge: NativeBridge;
  onSnapshotChanged(snapshot: AppSnapshot): void;
}

export function useSettingsController({
  bridge,
  onSnapshotChanged,
}: SettingsControllerOptions) {
  const [activity, setActivity] = useState<
    "idle" | "saving" | "checking" | "installing"
  >("idle");
  const [error, setError] = useState<string | null>(null);

  const updateDesktopSettings = async (settings: DesktopSettings) => {
    setActivity("saving");
    setError(null);
    try {
      onSnapshotChanged(await bridge.updateSettings({ settings }));
    } catch {
      setError("Formation Lap could not save these local desktop settings.");
    } finally {
      setActivity("idle");
    }
  };

  const checkUpdates = async () => {
    setActivity("checking");
    setError(null);
    try {
      onSnapshotChanged(await bridge.checkUpdates());
    } catch {
      setError("Formation Lap could not complete the trusted update checks.");
    } finally {
      setActivity("idle");
    }
  };

  const installFormationLapUpdate = async () => {
    setActivity("installing");
    setError(null);
    try {
      onSnapshotChanged(await bridge.installFormationLapUpdate());
    } catch (error) {
      setError(
        commandErrorMessage(
          error,
          "Formation Lap could not download and start the verified update.",
        ),
      );
    } finally {
      setActivity("idle");
    }
  };

  return {
    activity,
    error,
    clearError: () => setError(null),
    updateDesktopSettings,
    checkUpdates,
    installFormationLapUpdate,
  };
}
