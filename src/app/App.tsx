import { useEffect, useState, type FormEvent } from "react";
import markUrl from "../assets/formation-lap-mark.svg";
import type { AppSnapshot, LaunchSource } from "../generated/bindings";
import type { NativeBridge } from "../native-bridge/native-bridge";
import {
  CheckIcon,
  DashboardIcon,
  FlagIcon,
  PlusIcon,
  PulseIcon,
  SettingsIcon,
} from "../ui/icons";
import "./app.css";

interface AppProps {
  bridge: NativeBridge;
}

type SnapshotState =
  | { kind: "loading" }
  | { kind: "ready"; snapshot: AppSnapshot }
  | { kind: "error" };

type WorkspaceView = "dashboard" | "new-profile";
type PrimarySimSource = "direct" | "steam";

export function App({ bridge }: AppProps) {
  const [state, setState] = useState<SnapshotState>({ kind: "loading" });
  const [view, setView] = useState<WorkspaceView>("dashboard");
  const [profileName, setProfileName] = useState("");
  const [primarySimName, setPrimarySimName] = useState("");
  const [primarySimSource, setPrimarySimSource] =
    useState<PrimarySimSource>("direct");
  const [sourceValue, setSourceValue] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    bridge
      .getAppSnapshot()
      .then((snapshot) => {
        if (active) {
          setState({ kind: "ready", snapshot });
        }
      })
      .catch(() => {
        if (active) {
          setState({ kind: "error" });
        }
      });

    return () => {
      active = false;
    };
  }, [bridge]);

  const snapshot = state.kind === "ready" ? state.snapshot : null;
  const selectedProfile = snapshot?.selectedProfile ?? null;
  const applicationName = snapshot?.applicationName ?? "Formation Lap";

  const openNewProfile = () => {
    setFormError(null);
    setView("new-profile");
  };

  const selectProfile = async (profileId: string) => {
    try {
      const nextSnapshot = await bridge.selectProfile({
        profileId,
      });
      setState({ kind: "ready", snapshot: nextSnapshot });
      setView("dashboard");
    } catch {
      setState({ kind: "error" });
    }
  };

  const createProfile = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsSaving(true);
    setFormError(null);

    try {
      let nextSnapshot = await bridge.createProfile({
        name: profileName,
        primarySimName,
      });
      if (nextSnapshot.selectedProfile) {
        const profile = structuredClone(nextSnapshot.selectedProfile);
        const source: LaunchSource =
          primarySimSource === "steam"
            ? {
                kind: "steam",
                appId: Number.parseInt(sourceValue, 10) || 0,
              }
            : {
                kind: "directExecutable",
                executablePath: sourceValue,
              };
        profile.primarySim.launchRecipe.source = source;
        profile.primarySim.pathNeedsRepair =
          source.kind === "directExecutable" &&
          source.executablePath.length === 0;
        nextSnapshot = await bridge.saveProfile({ profile });
      }
      setState({ kind: "ready", snapshot: nextSnapshot });
      setView("dashboard");
      setProfileName("");
      setPrimarySimName("");
      setSourceValue("");
    } catch {
      setFormError(
        "The Racing Profile could not be created. Review the profile details and try again.",
      );
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="app-shell">
      <aside className="sidebar" aria-label="Primary">
        <div className="brand-lockup">
          <img src={markUrl} alt="" className="brand-mark" />
          <span>{applicationName}</span>
        </div>

        <div className="sidebar-section">
          <p className="sidebar-label">Profiles</p>
          <nav aria-label="Workspace">
            <button
              type="button"
              className={`nav-item ${view === "dashboard" ? "nav-item-active" : ""}`}
              aria-current={view === "dashboard" ? "page" : undefined}
              onClick={() => setView("dashboard")}
            >
              <span className="nav-icon">
                <DashboardIcon />
              </span>
              Dashboard
            </button>
          </nav>

          {snapshot && snapshot.profiles.length > 0 ? (
            <nav className="profile-nav" aria-label="Racing Profiles">
              {snapshot.profiles.map((profile) => {
                const isSelected =
                  view === "dashboard" && selectedProfile?.id === profile.id;
                return (
                  <button
                    type="button"
                    className={`profile-nav-item ${isSelected ? "profile-nav-item-active" : ""}`}
                    aria-current={isSelected ? "page" : undefined}
                    key={profile.id}
                    onClick={() => void selectProfile(profile.id)}
                  >
                    <span className="profile-nav-icon">
                      <FlagIcon />
                    </span>
                    <span>
                      <strong>{profile.name}</strong>
                      <small>{profile.primarySimName}</small>
                    </span>
                  </button>
                );
              })}
            </nav>
          ) : (
            <div className="profile-empty">
              <span className="profile-empty-icon">
                <FlagIcon />
              </span>
              <span>
                <strong>No Racing Profiles</strong>
                <small>Create one to prepare a Session.</small>
              </span>
            </div>
          )}

          <button
            type="button"
            className="new-profile-button"
            disabled={state.kind !== "ready"}
            onClick={openNewProfile}
          >
            <PlusIcon />
            New profile
          </button>
        </div>

        <nav className="utility-nav" aria-label="Utilities">
          <button type="button" className="nav-item" disabled>
            <SettingsIcon />
            Settings
          </button>
          <button type="button" className="nav-item" disabled>
            <PulseIcon />
            Diagnostics
          </button>
        </nav>
      </aside>

      <main className="workspace">
        {view === "new-profile" && state.kind === "ready" ? (
          <ProfileWizard
            profileName={profileName}
            primarySimName={primarySimName}
            primarySimSource={primarySimSource}
            sourceValue={sourceValue}
            isSaving={isSaving}
            error={formError}
            onProfileNameChange={setProfileName}
            onPrimarySimNameChange={setPrimarySimName}
            onPrimarySimSourceChange={setPrimarySimSource}
            onSourceValueChange={setSourceValue}
            onCancel={() => setView("dashboard")}
            onSubmit={createProfile}
          />
        ) : (
          <Dashboard
            state={state}
            applicationName={applicationName}
            selectedProfile={selectedProfile}
            onCreateProfile={openNewProfile}
          />
        )}
      </main>
    </div>
  );
}

interface ProfileWizardProps {
  profileName: string;
  primarySimName: string;
  primarySimSource: PrimarySimSource;
  sourceValue: string;
  isSaving: boolean;
  error: string | null;
  onProfileNameChange(value: string): void;
  onPrimarySimNameChange(value: string): void;
  onPrimarySimSourceChange(value: PrimarySimSource): void;
  onSourceValueChange(value: string): void;
  onCancel(): void;
  onSubmit(event: FormEvent<HTMLFormElement>): void;
}

function ProfileWizard({
  profileName,
  primarySimName,
  primarySimSource,
  sourceValue,
  isSaving,
  error,
  onProfileNameChange,
  onPrimarySimNameChange,
  onPrimarySimSourceChange,
  onSourceValueChange,
  onCancel,
  onSubmit,
}: ProfileWizardProps) {
  return (
    <>
      <header className="workspace-header wizard-header">
        <div>
          <p className="eyebrow">First profile</p>
          <h1>Create your first Racing Profile</h1>
          <p className="workspace-summary">
            Name the setup, choose its Primary Sim, and confirm the locked
            game-last order.
          </p>
        </div>
        <button type="button" className="secondary-button" onClick={onCancel}>
          Cancel
        </button>
      </header>

      <form className="profile-wizard" onSubmit={onSubmit}>
        <div className="wizard-form">
          <section className="wizard-step" aria-labelledby="profile-step-title">
            <span className="step-index">01</span>
            <div>
              <h2 id="profile-step-title">Profile identity</h2>
              <p>Use a name you will recognize from the sidebar.</p>
              <label className="field">
                <span>Profile name</span>
                <input
                  autoFocus
                  required
                  value={profileName}
                  onChange={(event) =>
                    onProfileNameChange(event.currentTarget.value)
                  }
                  placeholder="Le Mans evening"
                />
              </label>
            </div>
          </section>

          <section className="wizard-step" aria-labelledby="sim-step-title">
            <span className="step-index">02</span>
            <div>
              <h2 id="sim-step-title">Primary Sim</h2>
              <p>
                The Primary Sim always launches after Supporting Applications.
              </p>
              <div className="field-grid">
                <label className="field">
                  <span>Primary Sim name</span>
                  <input
                    required
                    value={primarySimName}
                    onChange={(event) =>
                      onPrimarySimNameChange(event.currentTarget.value)
                    }
                    placeholder="Le Mans Ultimate"
                  />
                </label>
                <label className="field">
                  <span>Primary Sim source</span>
                  <select
                    value={primarySimSource}
                    onChange={(event) =>
                      onPrimarySimSourceChange(
                        event.currentTarget.value as PrimarySimSource,
                      )
                    }
                  >
                    <option value="direct">Direct executable</option>
                    <option value="steam">Steam</option>
                  </select>
                </label>
              </div>
              <label className="field">
                <span>
                  {primarySimSource === "steam"
                    ? "Steam App ID"
                    : "Executable path"}
                </span>
                <input
                  inputMode={primarySimSource === "steam" ? "numeric" : "text"}
                  value={sourceValue}
                  onChange={(event) =>
                    onSourceValueChange(event.currentTarget.value)
                  }
                  placeholder={
                    primarySimSource === "steam"
                      ? "2399420"
                      : String.raw`C:\Games\Le Mans Ultimate\LMU.exe`
                  }
                />
                <small>
                  You can leave this blank and repair the path in the profile
                  editor.
                </small>
              </label>
            </div>
          </section>

          {error && (
            <p className="form-error" role="alert">
              {error}
            </p>
          )}

          <div className="wizard-actions">
            <button
              type="submit"
              className="primary-button"
              disabled={isSaving}
            >
              {isSaving ? "Creating profile…" : "Create Racing Profile"}
            </button>
          </div>
        </div>

        <aside className="wizard-preview" aria-labelledby="preview-title">
          <div className="preview-heading">
            <span className="step-index">03</span>
            <div>
              <p className="eyebrow">Review</p>
              <h2 id="preview-title">Startup order</h2>
            </div>
          </div>
          <div className="order-preview">
            <div className="order-empty">
              <PlusIcon />
              <span>
                <strong>Supporting Applications</strong>
                <small>Add and order them after creating the profile.</small>
              </span>
            </div>
            <div className="order-divider" aria-hidden="true">
              <span />
              <small>Launches last</small>
            </div>
            <div className="game-order-row">
              <span className="game-order-icon">
                <FlagIcon />
              </span>
              <span>
                <strong>{primarySimName || "Primary Sim"}</strong>
                <small>
                  {primarySimSource === "steam"
                    ? "Steam source"
                    : "Direct executable"}
                </small>
              </span>
              <span className="locked-label">Locked</span>
            </div>
          </div>
          <p className="preview-note">
            The game-last rule keeps preparation predictable and cannot be
            reordered.
          </p>
        </aside>
      </form>
    </>
  );
}

interface DashboardProps {
  state: SnapshotState;
  applicationName: string;
  selectedProfile: AppSnapshot["selectedProfile"];
  onCreateProfile(): void;
}

function Dashboard({
  state,
  applicationName,
  selectedProfile,
  onCreateProfile,
}: DashboardProps) {
  const pageTitle = selectedProfile?.name ?? applicationName;

  return (
    <>
      <header className="workspace-header">
        <div>
          <p className="eyebrow">
            {selectedProfile ? "Racing Profile" : "Profile setup"}
          </p>
          <h1>{pageTitle}</h1>
          <p className="workspace-summary">
            {selectedProfile
              ? `${selectedProfile.primarySim.name} launches last after every Supporting Application.`
              : "Prepare every Supporting Application, then launch the Primary Sim."}
          </p>
        </div>
        <button
          type="button"
          className="primary-button"
          disabled
          aria-describedby="start-session-requirement"
        >
          Start session
        </button>
      </header>

      {state.kind === "loading" && (
        <section className="foundation-card" aria-labelledby="foundation-title">
          <BrandArt />
          <div className="foundation-copy" role="status">
            <p className="eyebrow">Local native state</p>
            <h2 id="foundation-title">Loading Formation Lap</h2>
            <p>Connecting the window to the native application.</p>
          </div>
        </section>
      )}

      {state.kind === "error" && (
        <section className="foundation-card" aria-labelledby="foundation-title">
          <BrandArt />
          <div className="foundation-copy" role="alert">
            <p className="eyebrow">Local native state</p>
            <h2 id="foundation-title">Formation Lap could not start</h2>
            <p>Close the window and open Formation Lap again.</p>
          </div>
        </section>
      )}

      {state.kind === "ready" && !selectedProfile && (
        <section className="foundation-card" aria-labelledby="foundation-title">
          <BrandArt />
          <div className="foundation-copy">
            <p className="eyebrow">Local profile library</p>
            <h2 id="foundation-title">Secure foundation ready</h2>
            <p id="start-session-requirement">
              Create a Racing Profile to define one Primary Sim and its ordered
              Supporting Applications.
            </p>
            <button
              type="button"
              className="secondary-button foundation-action"
              onClick={onCreateProfile}
            >
              Create Racing Profile
            </button>
          </div>
        </section>
      )}

      {state.kind === "ready" && selectedProfile && (
        <section className="profile-dashboard" aria-labelledby="sequence-title">
          <div className="profile-dashboard-heading">
            <div>
              <p className="eyebrow">Startup sequence</p>
              <h2 id="sequence-title">Ready to configure</h2>
            </div>
            <label className="vr-toggle">
              <input
                type="checkbox"
                checked={selectedProfile.vrEnabled}
                readOnly
              />
              <span>VR</span>
            </label>
          </div>

          <div className="application-list">
            {selectedProfile.supportingApplications.length === 0 ? (
              <div className="application-empty">
                <PlusIcon />
                <span>
                  <strong>No Supporting Applications</strong>
                  <small>Add telemetry, overlays, voice, or VR tools.</small>
                </span>
              </div>
            ) : (
              selectedProfile.supportingApplications.map(
                ({ application, requirement }) => (
                  <div className="application-row" key={application.id}>
                    <span className="application-icon">
                      <PulseIcon />
                    </span>
                    <span className="application-copy">
                      <strong>{application.name}</strong>
                      <small>{requirement}</small>
                    </span>
                    <span className="status-label">○ Stopped</span>
                  </div>
                ),
              )
            )}

            <div className="game-divider">
              <span />
              <small>Primary Sim · launches last</small>
            </div>
            <div className="application-row game-row">
              <span className="application-icon game-icon">
                <FlagIcon />
              </span>
              <span className="application-copy">
                <strong>{selectedProfile.primarySim.name}</strong>
                <small>
                  {selectedProfile.primarySim.pathNeedsRepair
                    ? "Executable path needs repair"
                    : "Launch Recipe ready"}
                </small>
              </span>
              <span className="status-label">○ Stopped</span>
            </div>
          </div>
          <p id="start-session-requirement" className="profile-guidance">
            Edit this profile to add Supporting Applications and repair launch
            details before starting a Session.
          </p>
        </section>
      )}

      <footer className="workspace-status">
        <div className="status-message">
          <CheckIcon />
          <span>
            <strong>
              {selectedProfile
                ? "Racing Profile saved"
                : "Secure foundation ready"}
            </strong>
            <small>No Session active</small>
          </span>
        </div>
        <span className="utility-data">M2 · LOCAL</span>
      </footer>
    </>
  );
}

function BrandArt() {
  return (
    <div className="foundation-art" aria-hidden="true">
      <span className="quiet-orbit quiet-orbit-large" />
      <span className="quiet-orbit quiet-orbit-small" />
      <img src={markUrl} alt="" />
    </div>
  );
}
