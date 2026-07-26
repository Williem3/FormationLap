import type {
  AppSnapshot,
  DesktopSettings,
  UpdateSnapshot,
} from "../../generated/bindings";
import { updateStatusLabel } from "../../ui/update-status";

export interface SettingsScreenProps {
  settings: DesktopSettings;
  updates: UpdateSnapshot;
  sessionState: AppSnapshot["session"]["state"];
  isSaving: boolean;
  error: string | null;
  onChange(settings: DesktopSettings): void;
  onCheckUpdates(): void;
  onOpenDiagnostics(): void;
  onQuit(): void;
}

export function SettingsScreen({
  settings,
  updates,
  sessionState,
  isSaving,
  error,
  onChange,
  onCheckUpdates,
  onOpenDiagnostics,
  onQuit,
}: SettingsScreenProps) {
  const update = (change: Partial<DesktopSettings>) =>
    onChange({ ...settings, ...change });

  return (
    <div className="settings-screen">
      <header className="workspace-header settings-header">
        <div>
          <p className="eyebrow">Local preferences</p>
          <h1>Settings</h1>
          <p className="workspace-summary">
            Profiles and usage data stay on this PC. Online checks run only when
            requested or explicitly enabled.
          </p>
        </div>
        <span className="settings-save-state" role="status">
          {isSaving ? "Saving…" : "Saved locally"}
        </span>
      </header>

      {error && (
        <p className="form-error settings-error" role="alert">
          {error}
        </p>
      )}

      <div className="settings-grid">
        <section className="settings-group" aria-labelledby="general-settings">
          <div className="settings-group-heading">
            <p className="eyebrow">Desktop</p>
            <h2 id="general-settings">General</h2>
          </div>
          <label className="settings-row">
            <span>
              <strong>Start with Windows</strong>
              <small>
                Opens minimized to the tray. Racing Profiles never auto-start.
              </small>
            </span>
            <input
              type="checkbox"
              checked={settings.startWithWindows}
              disabled={isSaving}
              onChange={(event) =>
                update({ startWithWindows: event.currentTarget.checked })
              }
            />
          </label>
        </section>

        <section
          className="settings-group"
          aria-labelledby="appearance-settings"
        >
          <div className="settings-group-heading">
            <p className="eyebrow">Interface</p>
            <h2 id="appearance-settings">Appearance</h2>
          </div>
          <div className="settings-row settings-row-stacked">
            <span>
              <strong>Theme</strong>
              <small>Follow Windows or keep a fixed local theme.</small>
            </span>
            <div className="theme-options" role="group" aria-label="Theme">
              {(["system", "light", "dark"] as const).map((theme) => (
                <button
                  key={theme}
                  type="button"
                  className={
                    settings.theme === theme ? "theme-option-active" : ""
                  }
                  aria-pressed={settings.theme === theme}
                  disabled={isSaving}
                  onClick={() => update({ theme })}
                >
                  {theme[0]?.toUpperCase()}
                  {theme.slice(1)}
                </button>
              ))}
            </div>
          </div>
          <label className="settings-row">
            <span>
              <strong>Reduce motion</strong>
              <small>Stops progress and transition animation.</small>
            </span>
            <input
              type="checkbox"
              checked={settings.reduceMotion}
              disabled={isSaving}
              onChange={(event) =>
                update({ reduceMotion: event.currentTarget.checked })
              }
            />
          </label>
        </section>

        <section className="settings-group" aria-labelledby="update-settings">
          <div className="settings-group-heading">
            <p className="eyebrow">Release channel</p>
            <h2 id="update-settings">Updates</h2>
          </div>
          <label className="settings-row">
            <span>
              <strong>Automatic daily checks</strong>
              <small>
                Off by default. When enabled, checks at most daily may contact
                Formation Lap and curated application providers through GitHub
                Releases, Winget, and SimHub’s official site.
              </small>
            </span>
            <input
              type="checkbox"
              aria-label="Automatic daily checks"
              checked={settings.automaticUpdateChecks}
              disabled={isSaving}
              onChange={(event) =>
                update({
                  automaticUpdateChecks: event.currentTarget.checked,
                })
              }
            />
          </label>
          <div className="settings-row settings-row-stacked">
            <span>
              <strong>Signed release channel</strong>
              <small>
                Formation Lap installs only verified first-party updates.
                Third-party applications remain notification-only.
              </small>
            </span>
            <div
              className="theme-options"
              role="group"
              aria-label="Signed release channel"
            >
              {(["stable", "beta"] as const).map((channel) => (
                <button
                  key={channel}
                  type="button"
                  className={
                    settings.updateChannel === channel
                      ? "theme-option-active"
                      : ""
                  }
                  aria-pressed={settings.updateChannel === channel}
                  disabled={isSaving}
                  onClick={() => update({ updateChannel: channel })}
                >
                  {channel.charAt(0).toUpperCase()}
                  {channel.slice(1)}
                </button>
              ))}
            </div>
          </div>
          <div className="settings-row">
            <span>
              <strong>{updateStatusLabel(updates.formationLap)}</strong>
              <small>
                {sessionState === "idle"
                  ? "Check now consents to one direct check of the named providers."
                  : "Checks resume when the Session is idle."}
              </small>
            </span>
            <button
              type="button"
              className="secondary-button"
              disabled={isSaving || sessionState !== "idle"}
              onClick={onCheckUpdates}
            >
              Check now
            </button>
          </div>
        </section>

        <section
          className="settings-group"
          aria-labelledby="race-safe-settings"
        >
          <div className="settings-group-heading">
            <p className="eyebrow">While driving</p>
            <h2 id="race-safe-settings">Race-safe behavior</h2>
          </div>
          <div className="settings-row">
            <span>
              <strong>Suppress unsolicited disruptions</strong>
              <small>
                Updates and non-critical summaries wait until the Primary Sim
                exits.
              </small>
            </span>
            <span className="settings-value status-enabled">On</span>
          </div>
        </section>

        <section className="settings-group" aria-labelledby="privacy-settings">
          <div className="settings-group-heading">
            <p className="eyebrow">Local only</p>
            <h2 id="privacy-settings">Data &amp; privacy</h2>
          </div>
          <div className="settings-row">
            <span>
              <strong>No telemetry upload</strong>
              <small>
                Profiles, process observations, logs, and discovery results stay
                on this PC.
              </small>
            </span>
            <button
              type="button"
              className="secondary-button"
              onClick={onOpenDiagnostics}
            >
              Export diagnostics
            </button>
          </div>
        </section>

        <section
          className="settings-group settings-group-advanced"
          aria-labelledby="advanced-settings"
        >
          <div className="settings-group-heading">
            <p className="eyebrow">Maintenance</p>
            <h2 id="advanced-settings">Advanced</h2>
          </div>
          <div className="settings-row">
            <span>
              <strong>Bounded backups and logs</strong>
              <small>
                The last valid settings are backed up locally; diagnostic logs
                rotate at a fixed size.
              </small>
            </span>
            <button type="button" className="danger-button" onClick={onQuit}>
              Quit Formation Lap…
            </button>
          </div>
        </section>
      </div>
    </div>
  );
}
