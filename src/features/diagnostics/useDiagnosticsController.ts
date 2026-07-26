import { useState } from "react";
import type { DiagnosticExport } from "../../generated/bindings";
import type { NativeBridge } from "../../native-bridge/native-bridge";

interface DiagnosticsControllerOptions {
  bridge: NativeBridge;
  onOpen(): void;
}

export function useDiagnosticsController({
  bridge,
  onOpen,
}: DiagnosticsControllerOptions) {
  const [diagnostics, setDiagnostics] = useState<DiagnosticExport | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const openDiagnostics = async () => {
    onOpen();
    setIsLoading(true);
    setError(null);
    try {
      setDiagnostics(await bridge.exportDiagnostics());
    } catch {
      setError("Formation Lap could not export local diagnostics.");
    } finally {
      setIsLoading(false);
    }
  };

  return { diagnostics, isLoading, error, openDiagnostics };
}
