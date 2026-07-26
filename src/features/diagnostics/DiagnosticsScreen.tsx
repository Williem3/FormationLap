import type { DiagnosticExport } from "../../generated/bindings";

export interface DiagnosticsScreenProps {
  diagnostics: DiagnosticExport | null;
  isLoading: boolean;
  error: string | null;
  onRefresh(): void;
}

export function DiagnosticsScreen({
  diagnostics,
  isLoading,
  error,
  onRefresh,
}: DiagnosticsScreenProps) {
  return (
    <div className="diagnostics-screen">
      <header className="workspace-header settings-header">
        <div>
          <p className="eyebrow">Local support bundle</p>
          <h1>Diagnostics</h1>
          <p className="workspace-summary">
            Review the sanitized evidence before copying it. Formation Lap does
            not upload this export.
          </p>
        </div>
        <button
          type="button"
          className="secondary-button"
          disabled={isLoading}
          onClick={onRefresh}
        >
          {isLoading ? "Refreshing…" : "Refresh export"}
        </button>
      </header>
      {error && (
        <p className="form-error settings-error" role="alert">
          {error}
        </p>
      )}
      <section className="diagnostic-export" aria-labelledby="diagnostic-title">
        <div>
          <p className="eyebrow">Sanitized JSON</p>
          <h2 id="diagnostic-title">Diagnostic export</h2>
        </div>
        <textarea
          aria-label="Diagnostic export"
          readOnly
          rows={24}
          value={
            diagnostics
              ? JSON.stringify(diagnostics, null, 2)
              : isLoading
                ? "Preparing local diagnostics…"
                : "No diagnostic export is available."
          }
        />
        <p>
          Includes application version, platform, local settings, Session state,
          counts, and a bounded command-event tail. Executable paths and profile
          contents are omitted.
        </p>
      </section>
    </div>
  );
}
