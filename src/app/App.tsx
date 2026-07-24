import { useEffect, useState } from "react";
import markUrl from "../assets/formation-lap-mark.svg";
import type { AppSnapshot } from "../generated/bindings";
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

export function App({ bridge }: AppProps) {
  const [state, setState] = useState<SnapshotState>({ kind: "loading" });

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

  const applicationName =
    state.kind === "ready" ? state.snapshot.applicationName : "Formation Lap";

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
              className="nav-item nav-item-active"
              aria-current="page"
            >
              <span className="nav-icon">
                <DashboardIcon />
              </span>
              Dashboard
            </button>
          </nav>

          <div className="profile-empty">
            <span className="profile-empty-icon">
              <FlagIcon />
            </span>
            <span>
              <strong>No Racing Profiles</strong>
              <small>Create one to prepare a Session.</small>
            </span>
          </div>

          <button type="button" className="new-profile-button" disabled>
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
        <header className="workspace-header">
          <div>
            <p className="eyebrow">Secure project foundation</p>
            <h1>{applicationName}</h1>
            <p className="workspace-summary">
              Prepare every Supporting Application, then launch the Primary Sim.
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

        <section className="foundation-card" aria-labelledby="foundation-title">
          <div className="foundation-art" aria-hidden="true">
            <span className="quiet-orbit quiet-orbit-large" />
            <span className="quiet-orbit quiet-orbit-small" />
            <img src={markUrl} alt="" />
          </div>

          {state.kind === "loading" && (
            <div className="foundation-copy" role="status">
              <p className="eyebrow">Local native state</p>
              <h2 id="foundation-title">Loading Formation Lap</h2>
              <p>Connecting the window to the native application.</p>
            </div>
          )}

          {state.kind === "error" && (
            <div className="foundation-copy" role="alert">
              <p className="eyebrow">Local native state</p>
              <h2 id="foundation-title">Formation Lap could not start</h2>
              <p>Close the window and open Formation Lap again.</p>
            </div>
          )}

          {state.kind === "ready" && (
            <div className="foundation-copy">
              <p className="eyebrow">Local native state</p>
              <h2 id="foundation-title">Secure foundation ready</h2>
              <p id="start-session-requirement">
                Add a Racing Profile before starting a Session. Profile setup
                arrives in the next milestone.
              </p>
              <div className="foundation-details">
                <span>Local-only state</span>
                <span>Native Windows window</span>
                <span>Typed native bridge</span>
              </div>
            </div>
          )}
        </section>

        <footer className="workspace-status">
          <div className="status-message">
            <CheckIcon />
            <span>
              <strong>Secure foundation ready</strong>
              <small>No Session active</small>
            </span>
          </div>
          <span className="utility-data">M1 · LOCAL</span>
        </footer>
      </main>
    </div>
  );
}
