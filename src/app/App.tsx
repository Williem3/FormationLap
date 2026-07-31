import { useEffect, useRef, useState } from "react";
import markUrl from "../assets/formation-lap-mark.svg";
import type { QuitDisposition } from "../generated/bindings";
import type { NativeBridge } from "../native-bridge/native-bridge";
import {
  DashboardIcon,
  FlagIcon,
  PlusIcon,
  PulseIcon,
  SettingsIcon,
} from "../ui/icons";
import type { SnapshotState } from "./app-types";
import { Dashboard } from "../features/dashboard/Dashboard";
import { DashboardDialogs } from "../features/dashboard/DashboardDialogs";
import { useDashboardController } from "../features/dashboard/useDashboardController";
import { DiagnosticsScreen } from "../features/diagnostics/DiagnosticsScreen";
import { useDiagnosticsController } from "../features/diagnostics/useDiagnosticsController";
import { ProfileEditor } from "../features/profiles/ProfileEditor";
import { ProfileDialogs } from "../features/profiles/ProfileDialogs";
import { ProfileWizard } from "../features/profiles/ProfileWizard";
import { SettingsScreen } from "../features/settings/SettingsScreen";
import { useSettingsController } from "../features/settings/useSettingsController";
import { ModalDialog } from "../ui/ModalDialog";
import { profileApplicationIcon } from "../ui/presentation";
import { useProfileWorkspace } from "../features/profiles/useProfileWorkspace";
import "./app.css";

interface AppProps {
  bridge: NativeBridge;
}

type WorkspaceView =
  "dashboard" | "new-profile" | "edit-profile" | "settings" | "diagnostics";
export function App({ bridge }: AppProps) {
  const [state, setState] = useState<SnapshotState>({ kind: "loading" });
  const [view, setView] = useState<WorkspaceView>(() =>
    import.meta.env.DEV &&
    ["m8-settings", "m9-settings"].includes(
      new URLSearchParams(window.location.search).get("preview") ?? "",
    )
      ? "settings"
      : "dashboard",
  );
  const [quitIsSaving, setQuitIsSaving] = useState(false);
  const [quitError, setQuitError] = useState<string | null>(null);
  const [isQuitOpen, setIsQuitOpen] = useState(false);
  const newProfileButton = useRef<HTMLButtonElement | null>(null);

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
  const selectedProfileNeedsReview =
    snapshot?.profiles.find((profile) => profile.id === selectedProfile?.id)
      ?.reviewStatus === "needsReview";
  const {
    dashboardIsBusy,
    dashboardError,
    setDashboardError,
    pendingProcessAction,
    setPendingProcessAction,
    outputApplication,
    setOutputApplication,
    gameLaunchDiagnostic,
    clearGameLaunchDiagnostic,
    startApplication,
    testGameLaunch,
    toggleDashboardVr,
    runSessionAction,
    requestProcessAction,
    confirmProcessAction,
    confirmNativeProcessAction,
    cancelProcessAction,
    installFormationLapUpdate,
  } = useDashboardController({
    bridge,
    selectedProfile,
    snapshot,
    onSnapshotChanged: (nextSnapshot) =>
      setState({ kind: "ready", snapshot: nextSnapshot }),
  });
  const {
    profileName,
    setProfileName,
    primarySimName,
    setPrimarySimName,
    primarySimSource,
    setPrimarySimSource,
    sourceValue,
    setSourceValue,
    discoveryState,
    recommendationState,
    selectedPrimarySimId,
    selectedSupportingIds,
    isManualEntry,
    profileDraft,
    setProfileDraft,
    duplicateName,
    setDuplicateName,
    isDuplicateOpen,
    setIsDuplicateOpen,
    isDeleteOpen,
    setIsDeleteOpen,
    isExportOpen,
    setIsExportOpen,
    exportDocument,
    isImportOpen,
    setIsImportOpen,
    importDocument,
    setImportDocument,
    profileIsSaving,
    profileError,
    setProfileError,
    openNewProfile,
    selectDiscoveredPrimarySim,
    enterManualPrimarySim,
    toggleSupportingApplication,
    cancelProfileWizard,
    selectProfile,
    createProfile,
    openProfileEditor,
    pickExecutablePath,
    saveProfile,
    openDuplicateProfile,
    duplicateProfile,
    deleteProfile,
    exportProfile,
    importProfile,
  } = useProfileWorkspace({
    bridge,
    selectedProfile,
    selectedProfileNeedsReview,
    onSnapshotChanged: (nextSnapshot) =>
      setState({ kind: "ready", snapshot: nextSnapshot }),
    onSnapshotError: () => setState({ kind: "error" }),
    onNavigate: setView,
    onProfileSelected: clearGameLaunchDiagnostic,
  });
  const applicationName = snapshot?.applicationName ?? "Formation Lap";
  const {
    activity: settingsActivity,
    error: settingsError,
    clearError: clearSettingsError,
    updateDesktopSettings,
    checkUpdates,
    installFormationLapUpdate: installFormationLapUpdateFromSettings,
  } = useSettingsController({
    bridge,
    onSnapshotChanged: (nextSnapshot) =>
      setState({ kind: "ready", snapshot: nextSnapshot }),
  });
  const {
    diagnostics: diagnosticExport,
    isLoading: isDiagnosticsLoading,
    error: diagnosticsError,
    openDiagnostics,
  } = useDiagnosticsController({
    bridge,
    onOpen: () => setView("diagnostics"),
  });
  useEffect(() => {
    if (!snapshot) {
      return;
    }
    document.documentElement.dataset.theme = snapshot.settings.theme;
    document.documentElement.dataset.reduceMotion = String(
      snapshot.settings.reduceMotion,
    );
  }, [snapshot]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void bridge
      .listenForQuitRequest(() => {
        setIsQuitOpen(true);
      })
      .then((cleanup) => {
        unlisten = cleanup;
      });
    return () => unlisten?.();
  }, [bridge]);

  const activeProcessKey =
    snapshot?.applicationProcesses
      .filter((process) => process.identity !== null)
      .map((process) => `${process.applicationId}:${process.status}`)
      .join("|") ?? "";

  useEffect(() => {
    if (activeProcessKey.length === 0) {
      return;
    }
    let active = true;
    let refreshing = false;
    const timer = window.setInterval(() => {
      if (refreshing) {
        return;
      }
      refreshing = true;
      void bridge
        .refreshProcesses()
        .then((nextSnapshot) => {
          if (active) {
            setState({ kind: "ready", snapshot: nextSnapshot });
          }
        })
        .catch(() => {
          if (active) {
            setDashboardError(
              "Formation Lap could not refresh local application status.",
            );
          }
        })
        .finally(() => {
          refreshing = false;
        });
    }, 3_000);

    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [activeProcessKey, bridge, setDashboardError]);

  const requestQuit = async (disposition: QuitDisposition) => {
    setQuitIsSaving(true);
    setQuitError(null);
    try {
      const nextSnapshot = await bridge.requestQuit({ disposition });
      setState({ kind: "ready", snapshot: nextSnapshot });
      setIsQuitOpen(false);
    } catch {
      setQuitError("Formation Lap could not apply the selected Quit action.");
    } finally {
      setQuitIsSaving(false);
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
                      {profileApplicationIcon(
                        profile.primarySimApplicationId ??
                          (profile.id === selectedProfile?.id
                            ? selectedProfile.primarySim.id
                            : ""),
                        snapshot.applicationIcons ?? [],
                        <FlagIcon />,
                      )}
                    </span>
                    <span>
                      <strong>{profile.name}</strong>
                      <small>
                        {profile.primarySimName}
                        {profile.reviewStatus === "needsReview"
                          ? " · Review required"
                          : ""}
                      </small>
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
            ref={newProfileButton}
            type="button"
            className="new-profile-button"
            disabled={state.kind !== "ready"}
            onClick={openNewProfile}
          >
            <PlusIcon />
            New profile
          </button>
          <button
            type="button"
            className="import-profile-button"
            disabled={state.kind !== "ready"}
            onClick={() => {
              setProfileError(null);
              setImportDocument("");
              setIsImportOpen(true);
            }}
          >
            Import profile
          </button>
        </div>

        <nav className="utility-nav" aria-label="Utilities">
          <button
            type="button"
            className={`nav-item ${view === "settings" ? "nav-item-active" : ""}`}
            aria-current={view === "settings" ? "page" : undefined}
            disabled={state.kind !== "ready"}
            onClick={() => {
              clearSettingsError();
              setView("settings");
            }}
          >
            <SettingsIcon />
            Settings
          </button>
          <button
            type="button"
            className={`nav-item ${view === "diagnostics" ? "nav-item-active" : ""}`}
            aria-current={view === "diagnostics" ? "page" : undefined}
            disabled={state.kind !== "ready"}
            onClick={() => void openDiagnostics()}
          >
            <PulseIcon />
            Diagnostics
          </button>
          <button
            type="button"
            className="nav-item quit-nav-item"
            disabled={state.kind !== "ready"}
            onClick={() => {
              setQuitError(null);
              setIsQuitOpen(true);
            }}
          >
            <FlagIcon />
            Quit…
          </button>
        </nav>
      </aside>

      <main className="workspace">
        {view === "settings" && state.kind === "ready" ? (
          <SettingsScreen
            settings={state.snapshot.settings}
            updates={state.snapshot.updates}
            sessionState={state.snapshot.session.state}
            activity={settingsActivity}
            error={settingsError}
            onChange={(settings) => void updateDesktopSettings(settings)}
            onCheckUpdates={() => void checkUpdates()}
            onInstallFormationLapUpdate={() =>
              void installFormationLapUpdateFromSettings()
            }
            onOpenDiagnostics={() => void openDiagnostics()}
            onQuit={() => {
              setIsQuitOpen(true);
            }}
          />
        ) : view === "diagnostics" && state.kind === "ready" ? (
          <DiagnosticsScreen
            diagnostics={diagnosticExport}
            isLoading={isDiagnosticsLoading}
            error={diagnosticsError}
            onRefresh={() => void openDiagnostics()}
          />
        ) : view === "new-profile" && state.kind === "ready" ? (
          <ProfileWizard
            profileName={profileName}
            primarySimName={primarySimName}
            primarySimSource={primarySimSource}
            sourceValue={sourceValue}
            discoveryState={discoveryState}
            recommendationState={recommendationState}
            selectedPrimarySimId={selectedPrimarySimId}
            selectedSupportingIds={selectedSupportingIds}
            isManualEntry={isManualEntry}
            isSaving={profileIsSaving}
            error={profileError}
            onProfileNameChange={setProfileName}
            onPrimarySimNameChange={setPrimarySimName}
            onPrimarySimSourceChange={setPrimarySimSource}
            onSourceValueChange={setSourceValue}
            onPickExecutablePath={pickExecutablePath}
            onSelectPrimarySim={selectDiscoveredPrimarySim}
            onEnterManual={enterManualPrimarySim}
            onToggleSupporting={toggleSupportingApplication}
            onCancel={cancelProfileWizard}
            onSubmit={createProfile}
          />
        ) : view === "edit-profile" &&
          state.kind === "ready" &&
          profileDraft ? (
          <ProfileEditor
            profile={profileDraft}
            applicationIcons={snapshot?.applicationIcons ?? []}
            needsReview={selectedProfileNeedsReview}
            isSaving={profileIsSaving}
            error={profileError}
            onPickExecutablePath={pickExecutablePath}
            onChange={setProfileDraft}
            onCancel={() => {
              setProfileDraft(null);
              setView("dashboard");
            }}
            onSubmit={saveProfile}
          />
        ) : (
          <Dashboard
            state={state}
            applicationName={applicationName}
            selectedProfile={selectedProfile}
            profileNeedsReview={selectedProfileNeedsReview}
            error={dashboardError}
            applicationIcons={snapshot?.applicationIcons ?? []}
            applicationProcesses={snapshot?.applicationProcesses ?? []}
            session={snapshot?.session ?? null}
            updates={snapshot?.updates ?? null}
            onlineChecksEnabled={
              snapshot?.settings.automaticUpdateChecks ?? false
            }
            isBusy={dashboardIsBusy}
            gameLaunchDiagnostic={gameLaunchDiagnostic}
            onCreateProfile={openNewProfile}
            onDeleteProfile={() => {
              setProfileError(null);
              setIsDeleteOpen(true);
            }}
            onDuplicateProfile={openDuplicateProfile}
            onEditProfile={openProfileEditor}
            onExportProfile={() => void exportProfile()}
            onStartApplication={(application) =>
              void startApplication(application)
            }
            onExitApplication={(application, processSnapshot) =>
              requestProcessAction("exit", application, processSnapshot)
            }
            onRestartApplication={(application, processSnapshot) =>
              requestProcessAction("restart", application, processSnapshot)
            }
            onForceStopApplication={(application, processSnapshot) =>
              requestProcessAction("force", application, processSnapshot)
            }
            onViewOutput={setOutputApplication}
            onTestGameLaunch={() => void testGameLaunch()}
            onVrEnabledChange={(vrEnabled) => void toggleDashboardVr(vrEnabled)}
            onStartSession={() => void runSessionAction("start")}
            onCancelStartup={() => void runSessionAction("cancel")}
            onCloseSession={() => void runSessionAction("close")}
            onAcceptRecovery={() => void runSessionAction("acceptRecovery")}
            onDismissRecovery={() => void runSessionAction("dismissRecovery")}
            onInstallFormationLapUpdate={() => void installFormationLapUpdate()}
          />
        )}
      </main>

      {isQuitOpen && (
        <ModalDialog
          labelledBy="quit-title"
          onClose={() => setIsQuitOpen(false)}
        >
          <p className="eyebrow">Explicit Quit</p>
          <h2 id="quit-title">
            {snapshot?.session.state === "idle"
              ? "Quit Formation Lap?"
              : "What should happen to this Session?"}
          </h2>
          <p>
            {snapshot?.session.state === "idle"
              ? "No Session is active. Formation Lap can exit now."
              : "Choose whether Formation Lap closes Session-owned applications or leaves every running application untouched."}
          </p>
          {quitError && (
            <p className="form-error" role="alert">
              {quitError}
            </p>
          )}
          <div className="dialog-actions quit-dialog-actions">
            <button
              type="button"
              className="secondary-button"
              onClick={() => setIsQuitOpen(false)}
            >
              Cancel
            </button>
            {snapshot?.session.state !== "idle" && (
              <button
                type="button"
                className="secondary-button"
                disabled={quitIsSaving}
                onClick={() => void requestQuit("leaveApplicationsRunning")}
              >
                Leave applications running
              </button>
            )}
            <button
              type="button"
              className="primary-button"
              disabled={quitIsSaving}
              onClick={() =>
                void requestQuit(
                  snapshot?.session.state === "idle"
                    ? "leaveApplicationsRunning"
                    : "closeSession",
                )
              }
            >
              {snapshot?.session.state === "idle"
                ? "Quit Formation Lap"
                : "Close Session and quit"}
            </button>
          </div>
        </ModalDialog>
      )}

      <DashboardDialogs
        snapshot={snapshot}
        controller={{
          pendingProcessAction,
          setPendingProcessAction,
          outputApplication,
          setOutputApplication,
          dashboardError,
          dashboardIsBusy,
          confirmProcessAction,
          confirmNativeProcessAction,
          cancelProcessAction,
        }}
      />
      <ProfileDialogs
        selectedProfile={selectedProfile}
        newProfileButton={newProfileButton}
        workspace={{
          duplicateName,
          setDuplicateName,
          isDuplicateOpen,
          setIsDuplicateOpen,
          isDeleteOpen,
          setIsDeleteOpen,
          isExportOpen,
          setIsExportOpen,
          exportDocument,
          isImportOpen,
          setIsImportOpen,
          importDocument,
          setImportDocument,
          profileIsSaving,
          profileError,
          duplicateProfile,
          deleteProfile,
          importProfile,
        }}
      />
    </div>
  );
}
