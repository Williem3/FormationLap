import { useState } from "react";
import type { AppSnapshot, DesktopSettings } from "../../generated/bindings";
import type { NativeBridge } from "../../native-bridge/native-bridge";

interface SettingsControllerOptions {
  bridge: NativeBridge;
  onSnapshotChanged(snapshot: AppSnapshot): void;
}

export function useSettingsController({
  bridge,
  onSnapshotChanged,
}: SettingsControllerOptions) {
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const updateDesktopSettings = async (settings: DesktopSettings) => {
    setIsSaving(true);
    setError(null);
    try {
      onSnapshotChanged(await bridge.updateSettings({ settings }));
    } catch {
      setError("Formation Lap could not save these local desktop settings.");
    } finally {
      setIsSaving(false);
    }
  };

  const checkUpdates = async () => {
    setIsSaving(true);
    setError(null);
    try {
      onSnapshotChanged(await bridge.checkUpdates());
    } catch {
      setError("Formation Lap could not complete the trusted update checks.");
    } finally {
      setIsSaving(false);
    }
  };

  return {
    isSaving,
    error,
    clearError: () => setError(null),
    updateDesktopSettings,
    checkUpdates,
  };
}
