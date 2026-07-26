import type { ReactNode } from "react";
import markUrl from "../../assets/formation-lap-mark.svg";
import type {
  ApplicationUpdateSnapshot,
  ApplicationProcessSnapshot,
  AppSnapshot,
  GameLaunchDiagnostic,
  ProfileApplication,
  SessionApplicationSnapshot,
  UpdateSnapshot,
} from "../../generated/bindings";
import type { SnapshotState } from "../../app/app-types";
import { CheckIcon, FlagIcon, PlusIcon, PulseIcon } from "../../ui/icons";
import { profileApplicationIcon } from "../../ui/presentation";
import { updateStatusLabel } from "../../ui/update-status";

export interface DashboardProps {
  state: SnapshotState;
  applicationName: string;
  selectedProfile: AppSnapshot["selectedProfile"];
  profileNeedsReview: boolean;
  error: string | null;
  applicationIcons: NonNullable<AppSnapshot["applicationIcons"]>;
  applicationProcesses: ApplicationProcessSnapshot[];
  session: AppSnapshot["session"] | null;
  updates: UpdateSnapshot | null;
  onlineChecksEnabled: boolean;
  isBusy: boolean;
  gameLaunchDiagnostic: GameLaunchDiagnostic | null;
  onCreateProfile(): void;
  onDeleteProfile(): void;
  onDuplicateProfile(): void;
  onEditProfile(): void;
  onExportProfile(): void;
  onStartApplication(application: ProfileApplication): void;
  onExitApplication(
    application: ProfileApplication,
    process: ApplicationProcessSnapshot,
  ): void;
  onRestartApplication(
    application: ProfileApplication,
    process: ApplicationProcessSnapshot,
  ): void;
  onForceStopApplication(
    application: ProfileApplication,
    process: ApplicationProcessSnapshot,
  ): void;
  onViewOutput(application: ProfileApplication): void;
  onTestGameLaunch(): void;
  onVrEnabledChange(vrEnabled: boolean): void;
  onStartSession(): void;
  onCancelStartup(): void;
  onCloseSession(): void;
  onAcceptRecovery(): void;
  onDismissRecovery(): void;
  onInstallFormationLapUpdate(): void;
}

export function Dashboard({
  state,
  applicationName,
  selectedProfile,
  profileNeedsReview,
  error,
  applicationIcons,
  applicationProcesses,
  session,
  updates,
  onlineChecksEnabled,
  isBusy,
  gameLaunchDiagnostic,
  onCreateProfile,
  onDeleteProfile,
  onDuplicateProfile,
  onEditProfile,
  onExportProfile,
  onStartApplication,
  onExitApplication,
  onRestartApplication,
  onForceStopApplication,
  onViewOutput,
  onTestGameLaunch,
  onVrEnabledChange,
  onStartSession,
  onCancelStartup,
  onCloseSession,
  onAcceptRecovery,
  onDismissRecovery,
  onInstallFormationLapUpdate,
}: DashboardProps) {
  const pageTitle = selectedProfile?.name ?? applicationName;
  const sessionState = session?.state ?? "idle";
  const profileIsLocked =
    sessionState !== "idle" && session?.activeProfileId === selectedProfile?.id;
  const lifecycleControlsLocked =
    !["idle", "active"].includes(sessionState) || isBusy;
  const forceStopAvailable =
    !isBusy && ["idle", "active", "closing"].includes(sessionState);
  const railApplications = selectedProfile
    ? [
        ...selectedProfile.supportingApplications.map(
          (supporting) => supporting.application,
        ),
        selectedProfile.primarySim,
      ]
    : [];
  const formationRailReady = formationRailIsReady(
    railApplications,
    applicationProcesses,
    session?.applications ?? [],
  );

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
        <div className="header-actions">
          {selectedProfile && (
            <button
              type="button"
              className="secondary-button"
              onClick={onDuplicateProfile}
            >
              Duplicate profile
            </button>
          )}
          {selectedProfile && (
            <button
              type="button"
              className="secondary-button"
              disabled={profileIsLocked}
              onClick={onEditProfile}
            >
              Edit profile
            </button>
          )}
          {selectedProfile && (
            <button
              type="button"
              className="secondary-button"
              onClick={onExportProfile}
            >
              Export profile
            </button>
          )}
          {selectedProfile && (
            <button
              type="button"
              className="secondary-button danger-text"
              disabled={profileIsLocked}
              onClick={onDeleteProfile}
            >
              Delete profile
            </button>
          )}
          <button
            type="button"
            className="primary-button"
            disabled={
              isBusy ||
              !selectedProfile ||
              (profileNeedsReview && sessionState === "idle") ||
              sessionState === "cancelling" ||
              sessionState === "closing" ||
              sessionState === "recoveryAvailable"
            }
            aria-describedby="start-session-requirement"
            onClick={
              sessionState === "starting"
                ? onCancelStartup
                : sessionState === "active"
                  ? onCloseSession
                  : onStartSession
            }
          >
            {sessionState === "starting"
              ? "Cancel startup"
              : sessionState === "active"
                ? "Close session"
                : sessionState === "cancelling"
                  ? "Cancelling startup…"
                  : sessionState === "closing"
                    ? "Closing session…"
                    : sessionState === "recoveryAvailable"
                      ? "Recovery available"
                      : "Start session"}
          </button>
        </div>
      </header>
      {error && (
        <p className="form-error dashboard-error" role="alert">
          {error}
        </p>
      )}

      {updates?.resultDeferred && sessionState !== "idle" && (
        <section className="update-notice update-notice-deferred" role="status">
          <div>
            <p className="eyebrow">Race-safe update check</p>
            <strong>Update advice will appear after this Session</strong>
          </div>
          <span>Network results stay quiet while the Primary Sim runs.</span>
        </section>
      )}

      {updates?.formationLap.kind === "updateAvailable" && (
        <section className="update-notice" role="status">
          <div>
            <p className="eyebrow">Verified Formation Lap release</p>
            <strong>
              Formation Lap {updates.formationLap.latestVersion} is available
            </strong>
            <span>
              Current {updates.formationLap.currentVersion} · signed installer
            </span>
          </div>
          <button
            type="button"
            className="secondary-button"
            disabled={isBusy || sessionState !== "idle"}
            onClick={onInstallFormationLapUpdate}
          >
            Install verified update
          </button>
        </section>
      )}

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
            <p>
              Local data stays on this PC. Automatic online checks are{" "}
              {onlineChecksEnabled ? "on" : "off"}; online checks may contact
              Formation Lap and curated application providers through GitHub
              Releases, Winget, and SimHub’s official site.
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
          {profileNeedsReview && (
            <section
              className="profile-review-offer"
              aria-labelledby="profile-review-title"
            >
              <div>
                <p className="eyebrow">Native launch quarantine</p>
                <h2 id="profile-review-title">
                  Review imported executable settings
                </h2>
                <span>
                  Session start stays blocked until paths, arguments, working
                  directories, elevation, monitored executables, and stop
                  recipes are reviewed.
                </span>
              </div>
              <button
                type="button"
                className="secondary-button"
                disabled={isBusy || profileIsLocked}
                onClick={onEditProfile}
              >
                Review profile configuration
              </button>
            </section>
          )}
          {sessionState === "recoveryAvailable" && (
            <div className="recovery-offer" role="status">
              <div>
                <strong>Previous Session found</strong>
                <span>
                  Resume monitoring only after Formation Lap verifies the local
                  Processes.
                </span>
              </div>
              <div className="recovery-actions">
                <button
                  type="button"
                  className="secondary-button"
                  disabled={isBusy}
                  onClick={onDismissRecovery}
                >
                  Dismiss
                </button>
                <button
                  type="button"
                  className="primary-button"
                  disabled={isBusy}
                  onClick={onAcceptRecovery}
                >
                  Resume monitoring
                </button>
              </div>
            </div>
          )}
          <div className="profile-dashboard-heading">
            <div>
              <h2 id="sequence-title">
                {formationRailReady
                  ? "And Away we go!"
                  : "Drivers Start Your Engines!"}
              </h2>
            </div>
            <div className="profile-dashboard-controls">
              <button
                type="button"
                className="secondary-button"
                disabled={isBusy || sessionState !== "idle"}
                onClick={onTestGameLaunch}
              >
                Test game launch
              </button>
              <label className="vr-toggle">
                <input
                  type="checkbox"
                  aria-label="VR"
                  checked={selectedProfile.vrEnabled}
                  disabled={isBusy || sessionState !== "idle"}
                  onChange={(event) =>
                    onVrEnabledChange(event.currentTarget.checked)
                  }
                />
                <span>VR</span>
              </label>
            </div>
          </div>

          {gameLaunchDiagnostic && (
            <section
              className="game-launch-result"
              role="status"
              aria-label="Test Game Launch result"
            >
              <div>
                <p className="eyebrow">Test Game Launch complete</p>
                <strong>{gameLaunchDiagnostic.profileName}</strong>
              </div>
              <dl>
                <div>
                  <dt>Target</dt>
                  <dd>
                    {gameLaunchDiagnostic.target.kind === "steam"
                      ? gameLaunchDiagnostic.target.uri
                      : gameLaunchDiagnostic.target.executableName}
                  </dd>
                </div>
                <div>
                  <dt>Observed Process</dt>
                  <dd>{gameLaunchDiagnostic.observedProcess}</dd>
                </div>
              </dl>
              <details className="game-launch-diagnostic">
                <summary>Copy diagnostic</summary>
                <textarea
                  aria-label="Test Game Launch diagnostic"
                  readOnly
                  rows={6}
                  value={JSON.stringify(gameLaunchDiagnostic, null, 2)}
                />
              </details>
            </section>
          )}

          <FormationRail
            applications={railApplications}
            applicationProcesses={applicationProcesses}
            sessionApplications={session?.applications ?? []}
          />

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
                  <ApplicationLifecycleRow
                    key={application.id}
                    application={application}
                    classification={requirement}
                    icon={profileApplicationIcon(
                      application.id,
                      applicationIcons,
                      <PulseIcon />,
                    )}
                    process={applicationProcesses.find(
                      (candidate) => candidate.applicationId === application.id,
                    )}
                    update={updates?.applications.find(
                      (candidate) => candidate.applicationId === application.id,
                    )}
                    isBusy={isBusy}
                    lifecycleControlsLocked={lifecycleControlsLocked}
                    forceStopAvailable={forceStopAvailable}
                    onStart={onStartApplication}
                    onExit={onExitApplication}
                    onRestart={onRestartApplication}
                    onForceStop={onForceStopApplication}
                    onViewOutput={onViewOutput}
                  />
                ),
              )
            )}

            <div className="game-divider">
              <span />
              <small>Primary Sim · launches last</small>
            </div>
            <ApplicationLifecycleRow
              application={selectedProfile.primarySim}
              classification="Primary Sim"
              icon={profileApplicationIcon(
                selectedProfile.primarySim.id,
                applicationIcons,
                <FlagIcon />,
              )}
              process={applicationProcesses.find(
                (candidate) =>
                  candidate.applicationId === selectedProfile.primarySim.id,
              )}
              update={undefined}
              isBusy={isBusy}
              lifecycleControlsLocked={lifecycleControlsLocked}
              forceStopAvailable={forceStopAvailable}
              isPrimary
              onStart={onStartApplication}
              onExit={onExitApplication}
              onRestart={onRestartApplication}
              onForceStop={onForceStopApplication}
              onViewOutput={onViewOutput}
            />
          </div>
          <p id="start-session-requirement" className="profile-guidance">
            {profileNeedsReview
              ? "Review and approve this imported executable configuration before starting a Session."
              : "Formation Lap starts Supporting Applications in this order, confirms the Primary Sim last, and preserves Pre-existing Processes."}
          </p>
          {sessionState === "idle" && session?.summary && (
            <section
              className="session-summary"
              aria-labelledby="session-summary-title"
            >
              <div>
                <p className="eyebrow">Post-session summary</p>
                <h3 id="session-summary-title">Session notes</h3>
              </div>
              <ul>
                {session.summary.events.map((event) => (
                  <li key={`${event.applicationId}-${event.kind}`}>
                    <strong>{event.name}</strong>
                    <span>
                      {event.kind === "launchFailed"
                        ? "Did not finish startup"
                        : "Exited during the Session"}
                    </span>
                  </li>
                ))}
              </ul>
            </section>
          )}
        </section>
      )}

      <footer className="workspace-status">
        <div className="status-message">
          <CheckIcon />
          <span>
            <strong>
              {selectedProfile
                ? sessionState === "active"
                  ? "Session active"
                  : sessionState === "starting"
                    ? "Starting Session"
                    : sessionState === "closing"
                      ? "Closing Session"
                      : "Racing Profile saved"
                : "Secure foundation ready"}
            </strong>
            <small>
              {sessionState === "idle"
                ? "No Session active"
                : sessionState === "active"
                  ? "Primary Sim running · race-safe mode"
                  : "Formation Rail follows native Session state"}
            </small>
          </span>
        </div>
        <span className="utility-data">
          Local data · Online checks {onlineChecksEnabled ? "on" : "off"}
        </span>
      </footer>
    </>
  );
}

const processStatusLabels: Record<
  ApplicationProcessSnapshot["status"],
  string
> = {
  starting: "Starting",
  running: "Running",
  runningPreExisting: "Running (pre-existing)",
  notResponding: "Not Responding",
  stopping: "Stopping",
  stopped: "Stopped",
  failed: "Failed",
};

const sessionApplicationStateLabels: Record<
  SessionApplicationSnapshot["state"],
  string
> = {
  pending: "Pending",
  starting: "Starting",
  running: "Running",
  runningPreExisting: "Running (pre-existing)",
  failed: "Failed",
  stopping: "Stopping",
  stopped: "Stopped",
  detached: "Detached",
};

type FormationRailTone = "danger" | "neutral" | "running" | "warm";

function formationRailTone(
  status:
    ApplicationProcessSnapshot["status"] | SessionApplicationSnapshot["state"],
): FormationRailTone {
  switch (status) {
    case "running":
    case "runningPreExisting":
      return "running";
    case "pending":
    case "starting":
    case "stopping":
    case "notResponding":
      return "warm";
    case "stopped":
    case "failed":
      return "danger";
    case "detached":
      return "neutral";
  }
}

function formationRailStatus(
  applicationId: string,
  applicationProcesses: ApplicationProcessSnapshot[],
  sessionApplications: SessionApplicationSnapshot[],
) {
  const sessionApplication = sessionApplications.find(
    (candidate) => candidate.applicationId === applicationId,
  );
  return (
    sessionApplication?.state ??
    applicationProcesses.find(
      (candidate) => candidate.applicationId === applicationId,
    )?.status ??
    "stopped"
  );
}

function formationRailIsReady(
  applications: ProfileApplication[],
  applicationProcesses: ApplicationProcessSnapshot[],
  sessionApplications: SessionApplicationSnapshot[],
) {
  return (
    applications.length > 0 &&
    applications.every(
      (application) =>
        formationRailTone(
          formationRailStatus(
            application.id,
            applicationProcesses,
            sessionApplications,
          ),
        ) === "running",
    )
  );
}

function FormationRail({
  applications,
  applicationProcesses,
  sessionApplications,
}: {
  applications: ProfileApplication[];
  applicationProcesses: ApplicationProcessSnapshot[];
  sessionApplications: SessionApplicationSnapshot[];
}) {
  return (
    <ol className="formation-rail" aria-label="Formation Rail">
      {applications.map((application) => {
        const sessionApplication = sessionApplications.find(
          (candidate) => candidate.applicationId === application.id,
        );
        const process = applicationProcesses.find(
          (candidate) => candidate.applicationId === application.id,
        );
        const state = sessionApplication?.state;
        const status = state ?? process?.status ?? "stopped";
        const label = state
          ? sessionApplicationStateLabels[state]
          : processStatusLabels[process?.status ?? "stopped"];
        const tone = formationRailTone(status);
        return (
          <li
            className={`rail-node rail-node-${status}`}
            data-rail-tone={tone}
            key={application.id}
          >
            <span className="race-light" aria-hidden="true" />
            <span>
              <strong>{application.name}</strong>
              <small>{label}</small>
            </span>
          </li>
        );
      })}
    </ol>
  );
}

interface ApplicationLifecycleRowProps {
  application: ProfileApplication;
  classification: string;
  icon: ReactNode;
  process: ApplicationProcessSnapshot | undefined;
  update: ApplicationUpdateSnapshot | undefined;
  isBusy: boolean;
  lifecycleControlsLocked: boolean;
  forceStopAvailable: boolean;
  isPrimary?: boolean;
  onStart(application: ProfileApplication): void;
  onExit(
    application: ProfileApplication,
    process: ApplicationProcessSnapshot,
  ): void;
  onRestart(
    application: ProfileApplication,
    process: ApplicationProcessSnapshot,
  ): void;
  onForceStop(
    application: ProfileApplication,
    process: ApplicationProcessSnapshot,
  ): void;
  onViewOutput(application: ProfileApplication): void;
}

function ApplicationLifecycleRow({
  application,
  classification,
  icon,
  process,
  update,
  isBusy,
  lifecycleControlsLocked,
  forceStopAvailable,
  isPrimary = false,
  onStart,
  onExit,
  onRestart,
  onForceStop,
  onViewOutput,
}: ApplicationLifecycleRowProps) {
  const status = process?.status ?? "stopped";
  const isActive =
    process?.identity !== null && process?.identity !== undefined;
  const hasOutput = process?.output !== null && process?.output !== undefined;
  const sourceLabel =
    application.launchRecipe.source.kind === "directExecutable"
      ? `${application.launchRecipe.consoleVisibility} console`
      : `Steam ${application.launchRecipe.source.appId}`;
  const startupFailure =
    status === "failed"
      ? `${application.name} exited during startup. Check its executable path and enter each launch argument on a separate line.`
      : null;

  return (
    <article
      className={`application-row lifecycle-row ${isPrimary ? "game-row" : ""}`}
      data-status={status}
    >
      <span
        className={`application-icon ${isPrimary ? "game-icon" : ""}`}
        aria-hidden="true"
      >
        {icon}
      </span>
      <span className="application-copy">
        <strong>{application.name}</strong>
        <small>
          {application.pathNeedsRepair
            ? "Executable path needs repair"
            : sourceLabel}
        </small>
        {update && (
          <small
            className={`update-state update-state-${update.status.kind}`}
            title={
              update.status.kind === "unknown"
                ? update.status.reason
                : undefined
            }
          >
            <span className="status-glyph" aria-hidden="true" />
            {updateStatusLabel(update.status)}
          </small>
        )}
        {startupFailure && (
          <small className="application-failure">{startupFailure}</small>
        )}
      </span>
      <span className="classification-label">{classification}</span>
      <span
        className={`status-label status-${status}`}
        role="status"
        aria-label={`${application.name}: ${processStatusLabels[status]}`}
      >
        <span className="status-glyph" aria-hidden="true" />
        {processStatusLabels[status]}
      </span>
      <span className="application-actions">
        <button
          type="button"
          className="tertiary-button output-button"
          disabled={!hasOutput}
          onClick={() => onViewOutput(application)}
        >
          {hasOutput ? "View Output" : "No Output"}
        </button>
        {!isActive ? (
          <button
            type="button"
            className="secondary-button compact-action"
            disabled={
              isBusy || lifecycleControlsLocked || application.pathNeedsRepair
            }
            onClick={() => onStart(application)}
            aria-label={`Start ${application.name}`}
          >
            Start
          </button>
        ) : status === "stopping" ? (
          <button
            type="button"
            className="danger-button compact-action"
            disabled={isBusy || !forceStopAvailable}
            onClick={() => onForceStop(application, process)}
            aria-label={`Force stop ${application.name}`}
          >
            Force stop
          </button>
        ) : (
          <>
            <button
              type="button"
              className="secondary-button compact-action"
              disabled={isBusy || lifecycleControlsLocked}
              onClick={() => onExit(application, process)}
              aria-label={`Exit ${application.name}`}
            >
              Exit
            </button>
            <button
              type="button"
              className="secondary-button compact-action"
              disabled={isBusy || lifecycleControlsLocked}
              onClick={() => onRestart(application, process)}
              aria-label={`Restart ${application.name}`}
            >
              Restart
            </button>
          </>
        )}
      </span>
    </article>
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
