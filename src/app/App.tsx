import { useEffect, useRef, useState, type FormEvent } from "react";
import markUrl from "../assets/formation-lap-mark.svg";
import type {
  ApplicationProcessSnapshot,
  DesktopSettings,
  DiagnosticExport,
  DiscoveredPrimarySim,
  GameLaunchDiagnostic,
  LaunchSource,
  ProfileApplication,
  RacingProfile,
  QuitDisposition,
} from "../generated/bindings";
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
import { DiagnosticsScreen } from "../features/diagnostics/DiagnosticsScreen";
import { ProfileEditor } from "../features/profiles/ProfileEditor";
import { ProfileWizard } from "../features/profiles/ProfileWizard";
import type {
  DiscoveryState,
  PrimarySimSource,
  ProfileApproval,
  RecommendationState,
} from "../features/profiles/profile-types";
import { SettingsScreen } from "../features/settings/SettingsScreen";
import { ModalDialog } from "../ui/ModalDialog";
import {
  commandErrorMessage,
  discoveredSupportingApplicationToProfile,
} from "../ui/presentation";
import "./app.css";

interface AppProps {
  bridge: NativeBridge;
}

type WorkspaceView =
  "dashboard" | "new-profile" | "edit-profile" | "settings" | "diagnostics";
type PendingProcessAction = {
  kind: "exit" | "restart" | "force";
  application: ProfileApplication;
  process: ApplicationProcessSnapshot;
};

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
  const [profileName, setProfileName] = useState("");
  const [primarySimName, setPrimarySimName] = useState("");
  const [primarySimSource, setPrimarySimSource] =
    useState<PrimarySimSource>("direct");
  const [sourceValue, setSourceValue] = useState("");
  const [discoveryState, setDiscoveryState] = useState<DiscoveryState>({
    kind: "idle",
  });
  const [recommendationState, setRecommendationState] =
    useState<RecommendationState>({ kind: "idle" });
  const [selectedPrimarySimId, setSelectedPrimarySimId] = useState<
    string | null
  >(null);
  const [selectedSupportingIds, setSelectedSupportingIds] = useState<string[]>(
    [],
  );
  const [isManualEntry, setIsManualEntry] = useState(false);
  const [profileDraft, setProfileDraft] = useState<RacingProfile | null>(null);
  const [duplicateName, setDuplicateName] = useState("");
  const [isDuplicateOpen, setIsDuplicateOpen] = useState(false);
  const [isDeleteOpen, setIsDeleteOpen] = useState(false);
  const [isExportOpen, setIsExportOpen] = useState(false);
  const [exportDocument, setExportDocument] = useState("");
  const [isImportOpen, setIsImportOpen] = useState(false);
  const [importDocument, setImportDocument] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);
  const [pendingProcessAction, setPendingProcessAction] =
    useState<PendingProcessAction | null>(null);
  const [outputApplication, setOutputApplication] =
    useState<ProfileApplication | null>(null);
  const [gameLaunchDiagnostic, setGameLaunchDiagnostic] =
    useState<GameLaunchDiagnostic | null>(null);
  const [diagnosticExport, setDiagnosticExport] =
    useState<DiagnosticExport | null>(null);
  const [isDiagnosticsLoading, setIsDiagnosticsLoading] = useState(false);
  const [isQuitOpen, setIsQuitOpen] = useState(false);
  const dialogReturnFocus = useRef<HTMLElement | null>(null);
  const newProfileButton = useRef<HTMLButtonElement | null>(null);
  const wasDialogOpen = useRef(false);
  const recommendationRequest = useRef(0);

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
  const applicationName = snapshot?.applicationName ?? "Formation Lap";
  const isDialogOpen =
    isDuplicateOpen ||
    isDeleteOpen ||
    isExportOpen ||
    isImportOpen ||
    pendingProcessAction !== null ||
    outputApplication !== null ||
    isQuitOpen;

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
        if (document.activeElement instanceof HTMLElement) {
          dialogReturnFocus.current = document.activeElement;
        }
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
            setFormError(
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
  }, [activeProcessKey, bridge]);

  useEffect(() => {
    if (wasDialogOpen.current && !isDialogOpen) {
      dialogReturnFocus.current?.focus();
      dialogReturnFocus.current = null;
    }
    wasDialogOpen.current = isDialogOpen;
  }, [isDialogOpen]);

  const rememberDialogTrigger = () => {
    if (document.activeElement instanceof HTMLElement) {
      dialogReturnFocus.current = document.activeElement;
    }
  };

  const openNewProfile = () => {
    setFormError(null);
    setProfileName("");
    setPrimarySimName("");
    setPrimarySimSource("direct");
    setSourceValue("");
    setSelectedPrimarySimId(null);
    setSelectedSupportingIds([]);
    setRecommendationState({ kind: "idle" });
    setIsManualEntry(false);
    setDiscoveryState({ kind: "loading" });
    setView("new-profile");
    void bridge
      .discoverApplications()
      .then((discovery) => {
        setDiscoveryState({ kind: "ready", snapshot: discovery });
        if (discovery.installedPrimarySims.length === 0) {
          setIsManualEntry(true);
        }
      })
      .catch(() => {
        setDiscoveryState({ kind: "error" });
        setIsManualEntry(true);
      });
  };

  const selectDiscoveredPrimarySim = (primarySim: DiscoveredPrimarySim) => {
    setSelectedPrimarySimId(primarySim.id);
    setPrimarySimName(primarySim.name);
    setIsManualEntry(false);
    setSelectedSupportingIds([]);
    if (primarySim.installation.kind === "steam") {
      setPrimarySimSource("steam");
      setSourceValue(String(primarySim.installation.appId));
    } else {
      setPrimarySimSource("direct");
      setSourceValue(primarySim.installation.executablePath);
    }

    const request = ++recommendationRequest.current;
    setRecommendationState({
      kind: "loading",
      primarySimName: primarySim.name,
    });
    void bridge
      .recommendApplications({ primarySimId: primarySim.id })
      .then((recommendations) => {
        if (request !== recommendationRequest.current) {
          return;
        }
        const installedIds = new Set(
          discoveryState.kind === "ready"
            ? discoveryState.snapshot.installedSupportingApplications.map(
                (application) => application.id,
              )
            : [],
        );
        setRecommendationState({
          kind: "ready",
          primarySimName: primarySim.name,
          recommendations: recommendations.filter((recommendation) =>
            installedIds.has(recommendation.id),
          ),
        });
      })
      .catch(() => {
        if (request === recommendationRequest.current) {
          setRecommendationState({
            kind: "error",
            primarySimName: primarySim.name,
          });
        }
      });
  };

  const enterManualPrimarySim = () => {
    recommendationRequest.current += 1;
    setSelectedPrimarySimId(null);
    setSelectedSupportingIds([]);
    setRecommendationState({ kind: "idle" });
    setPrimarySimName("");
    setPrimarySimSource("direct");
    setSourceValue("");
    setIsManualEntry(true);
  };

  const toggleSupportingApplication = (applicationId: string) => {
    setSelectedSupportingIds((selected) =>
      selected.includes(applicationId)
        ? selected.filter((id) => id !== applicationId)
        : [...selected, applicationId],
    );
  };

  const selectProfile = async (profileId: string) => {
    try {
      const nextSnapshot = await bridge.selectProfile({
        profileId,
      });
      setState({ kind: "ready", snapshot: nextSnapshot });
      setView("dashboard");
      setProfileDraft(null);
      setGameLaunchDiagnostic(null);
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
                selector: null,
              }
            : {
                kind: "directExecutable",
                executablePath: sourceValue,
              };
        profile.primarySim.launchRecipe.source = source;
        profile.primarySim.pathNeedsRepair =
          source.kind === "directExecutable" &&
          source.executablePath.length === 0;
        if (discoveryState.kind === "ready") {
          profile.supportingApplications =
            discoveryState.snapshot.installedSupportingApplications
              .filter((application) =>
                selectedSupportingIds.includes(application.id),
              )
              .map(discoveredSupportingApplicationToProfile);
        }
        nextSnapshot = await bridge.saveProfile({ profile });
      }
      setState({ kind: "ready", snapshot: nextSnapshot });
      setView("dashboard");
      setProfileName("");
      setPrimarySimName("");
      setSourceValue("");
      setDiscoveryState({ kind: "idle" });
      setRecommendationState({ kind: "idle" });
      setSelectedPrimarySimId(null);
      setSelectedSupportingIds([]);
      setIsManualEntry(false);
    } catch {
      setFormError(
        "The Racing Profile could not be created. Review the profile details and try again.",
      );
    } finally {
      setIsSaving(false);
    }
  };

  const openProfileEditor = () => {
    if (!selectedProfile) {
      return;
    }
    setFormError(null);
    setProfileDraft(structuredClone(selectedProfile));
    setView("edit-profile");
  };

  const pickExecutablePath = async (
    initialPath?: string | null,
  ): Promise<string | null> => {
    try {
      return await bridge.pickExecutablePath(initialPath);
    } catch {
      setFormError(
        "Formation Lap could not open the executable picker. Type the path or try again.",
      );
      return null;
    }
  };

  const saveProfile = async (
    event: FormEvent<HTMLFormElement>,
    approval?: ProfileApproval,
  ) => {
    event.preventDefault();
    if (!profileDraft) {
      return;
    }
    setIsSaving(true);
    setFormError(null);
    try {
      let nextSnapshot = await bridge.saveProfile({
        profile: profileDraft,
      });
      if (approval) {
        nextSnapshot = await bridge.approveProfile({
          profileId: profileDraft.id,
          configurationReviewed: approval.configurationReviewed,
          approvedPrivilegedApplicationIds:
            approval.approvedPrivilegedApplicationIds,
        });
      }
      setState({ kind: "ready", snapshot: nextSnapshot });
      setProfileDraft(null);
      setView("dashboard");
    } catch {
      setFormError(
        selectedProfileNeedsReview
          ? "The Racing Profile is still quarantined. Repair missing paths and approve every elevated or custom-stop entry."
          : "The Racing Profile could not be saved. Review the profile details and try again.",
      );
    } finally {
      setIsSaving(false);
    }
  };

  const openDuplicateProfile = () => {
    if (!selectedProfile) {
      return;
    }
    rememberDialogTrigger();
    setFormError(null);
    setDuplicateName(`${selectedProfile.name} Copy`);
    setIsDuplicateOpen(true);
  };

  const duplicateProfile = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selectedProfile) {
      return;
    }
    setIsSaving(true);
    setFormError(null);
    try {
      const nextSnapshot = await bridge.duplicateProfile({
        sourceProfileId: selectedProfile.id,
        name: duplicateName,
      });
      setState({ kind: "ready", snapshot: nextSnapshot });
      setIsDuplicateOpen(false);
      setDuplicateName("");
    } catch {
      setFormError(
        "The Racing Profile could not be duplicated. Choose a different name and try again.",
      );
    } finally {
      setIsSaving(false);
    }
  };

  const deleteProfile = async () => {
    if (!selectedProfile) {
      return;
    }
    setIsSaving(true);
    setFormError(null);
    try {
      const nextSnapshot = await bridge.deleteProfile({
        profileId: selectedProfile.id,
      });
      dialogReturnFocus.current = newProfileButton.current;
      setState({ kind: "ready", snapshot: nextSnapshot });
      setIsDeleteOpen(false);
    } catch {
      setFormError(
        "The Racing Profile could not be deleted. Close this dialog and try again.",
      );
    } finally {
      setIsSaving(false);
    }
  };

  const exportProfile = async () => {
    if (!selectedProfile) {
      return;
    }
    rememberDialogTrigger();
    setIsSaving(true);
    setFormError(null);
    setExportDocument("");
    setIsExportOpen(true);
    try {
      const document = await bridge.exportProfile({
        profileId: selectedProfile.id,
      });
      setExportDocument(document);
    } catch {
      setFormError(
        "The Racing Profile could not be exported. Close this dialog and try again.",
      );
    } finally {
      setIsSaving(false);
    }
  };

  const importProfile = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsSaving(true);
    setFormError(null);
    try {
      const nextSnapshot = await bridge.importProfile({
        document: importDocument,
      });
      setState({ kind: "ready", snapshot: nextSnapshot });
      setIsImportOpen(false);
      setImportDocument("");
    } catch {
      setFormError(
        "The Racing Profile could not be imported. Check the portable JSON and try again.",
      );
    } finally {
      setIsSaving(false);
    }
  };

  const startApplication = async (application: ProfileApplication) => {
    if (!selectedProfile) {
      return;
    }
    setIsSaving(true);
    setFormError(null);
    try {
      const nextSnapshot = await bridge.startApplication({
        profileId: selectedProfile.id,
        applicationId: application.id,
      });
      setState({ kind: "ready", snapshot: nextSnapshot });
    } catch (error) {
      setFormError(
        commandErrorMessage(
          error,
          `${application.name} could not start. Check its Launch Recipe and try again.`,
        ),
      );
    } finally {
      setIsSaving(false);
    }
  };

  const testGameLaunch = async () => {
    if (!selectedProfile) {
      return;
    }
    setIsSaving(true);
    setFormError(null);
    setGameLaunchDiagnostic(null);
    try {
      const diagnostic = await bridge.testGameLaunch({
        profileId: selectedProfile.id,
      });
      setGameLaunchDiagnostic(diagnostic);
    } catch {
      setFormError(
        "Test Game Launch could not start the Primary Sim. Review its Launch Recipe and try again.",
      );
    } finally {
      setIsSaving(false);
    }
  };

  const toggleDashboardVr = async (vrEnabled: boolean) => {
    if (!selectedProfile || snapshot?.session.state !== "idle") {
      return;
    }
    setIsSaving(true);
    setFormError(null);
    try {
      const profile = structuredClone(selectedProfile);
      profile.vrEnabled = vrEnabled;
      const nextSnapshot = await bridge.saveProfile({ profile });
      setState({ kind: "ready", snapshot: nextSnapshot });
      setGameLaunchDiagnostic(null);
    } catch {
      setFormError("Formation Lap could not remember the VR choice.");
    } finally {
      setIsSaving(false);
    }
  };

  const runSessionAction = async (
    action: "start" | "cancel" | "close" | "acceptRecovery" | "dismissRecovery",
  ) => {
    if (action === "start" && !selectedProfile) {
      return;
    }
    setIsSaving(true);
    setFormError(null);
    try {
      const nextSnapshot =
        action === "start"
          ? await bridge.startSession({ profileId: selectedProfile!.id })
          : action === "cancel"
            ? await bridge.cancelStartup()
            : action === "close"
              ? await bridge.closeSession()
              : action === "acceptRecovery"
                ? await bridge.acceptRecovery()
                : await bridge.dismissRecovery();
      setState({ kind: "ready", snapshot: nextSnapshot });
    } catch (error) {
      setFormError(
        commandErrorMessage(
          error,
          "Formation Lap could not complete the Session action.",
        ),
      );
    } finally {
      setIsSaving(false);
    }
  };

  const exitApplication = async (
    application: ProfileApplication,
    preExistingConfirmed: boolean,
  ) => {
    setIsSaving(true);
    setFormError(null);
    try {
      const nextSnapshot = await bridge.exitApplication({
        applicationId: application.id,
        preExistingConfirmed,
      });
      setState({ kind: "ready", snapshot: nextSnapshot });
      const nextProcess = nextSnapshot.applicationProcesses.find(
        (process) => process.applicationId === application.id,
      );
      if (nextProcess?.status === "stopping") {
        rememberDialogTrigger();
        setPendingProcessAction({
          kind: "force",
          application,
          process: nextProcess,
        });
      }
    } catch {
      setFormError(
        `${application.name} did not stop. Review its shutdown strategy and try again.`,
      );
    } finally {
      setIsSaving(false);
    }
  };

  const restartApplication = async (
    application: ProfileApplication,
    preExistingConfirmed: boolean,
  ) => {
    if (!selectedProfile) {
      return;
    }
    setIsSaving(true);
    setFormError(null);
    try {
      const nextSnapshot = await bridge.restartApplication({
        profileId: selectedProfile.id,
        applicationId: application.id,
        preExistingConfirmed,
      });
      setState({ kind: "ready", snapshot: nextSnapshot });
      const nextProcess = nextSnapshot.applicationProcesses.find(
        (process) => process.applicationId === application.id,
      );
      if (nextProcess?.status === "stopping") {
        rememberDialogTrigger();
        setPendingProcessAction({
          kind: "force",
          application,
          process: nextProcess,
        });
      }
    } catch {
      setFormError(
        `${application.name} could not restart. Check its Launch Recipe and try again.`,
      );
    } finally {
      setIsSaving(false);
    }
  };

  const forceStopApplication = async (
    application: ProfileApplication,
    processSnapshot: ApplicationProcessSnapshot,
  ) => {
    setIsSaving(true);
    setFormError(null);
    try {
      const nextSnapshot = await bridge.forceStopApplication({
        applicationId: application.id,
        preExistingConfirmed: processSnapshot.ownership === "preExisting",
        forceConfirmed: true,
      });
      setState({ kind: "ready", snapshot: nextSnapshot });
      setPendingProcessAction(null);
    } catch {
      setFormError(
        `${application.name} could not be force stopped. Try again or close it directly.`,
      );
    } finally {
      setIsSaving(false);
    }
  };

  const requestProcessAction = (
    kind: PendingProcessAction["kind"],
    application: ProfileApplication,
    processSnapshot: ApplicationProcessSnapshot,
  ) => {
    rememberDialogTrigger();
    if (kind === "force" || processSnapshot.ownership === "preExisting") {
      setPendingProcessAction({
        kind,
        application,
        process: processSnapshot,
      });
      return;
    }
    if (kind === "exit") {
      void exitApplication(application, false);
    } else {
      void restartApplication(application, false);
    }
  };

  const confirmProcessAction = () => {
    if (!pendingProcessAction) {
      return;
    }
    const { kind, application, process } = pendingProcessAction;
    setPendingProcessAction(null);
    if (kind === "exit") {
      void exitApplication(application, true);
    } else if (kind === "restart") {
      void restartApplication(application, true);
    } else {
      void forceStopApplication(application, process);
    }
  };

  const updateDesktopSettings = async (settings: DesktopSettings) => {
    setIsSaving(true);
    setFormError(null);
    try {
      const nextSnapshot = await bridge.updateSettings({ settings });
      setState({ kind: "ready", snapshot: nextSnapshot });
    } catch {
      setFormError(
        "Formation Lap could not save these local desktop settings.",
      );
    } finally {
      setIsSaving(false);
    }
  };

  const checkUpdates = async () => {
    setIsSaving(true);
    setFormError(null);
    try {
      const nextSnapshot = await bridge.checkUpdates();
      setState({ kind: "ready", snapshot: nextSnapshot });
    } catch {
      setFormError(
        "Formation Lap could not complete the trusted update checks.",
      );
    } finally {
      setIsSaving(false);
    }
  };

  const installFormationLapUpdate = async () => {
    setIsSaving(true);
    setFormError(null);
    try {
      const nextSnapshot = await bridge.installFormationLapUpdate();
      setState({ kind: "ready", snapshot: nextSnapshot });
    } catch {
      setFormError(
        "Formation Lap rejected the update or the Session is not idle.",
      );
    } finally {
      setIsSaving(false);
    }
  };

  const openDiagnostics = async () => {
    setView("diagnostics");
    setIsDiagnosticsLoading(true);
    setFormError(null);
    try {
      setDiagnosticExport(await bridge.exportDiagnostics());
    } catch {
      setFormError("Formation Lap could not export local diagnostics.");
    } finally {
      setIsDiagnosticsLoading(false);
    }
  };

  const requestQuit = async (disposition: QuitDisposition) => {
    setIsSaving(true);
    setFormError(null);
    try {
      const nextSnapshot = await bridge.requestQuit({ disposition });
      setState({ kind: "ready", snapshot: nextSnapshot });
      setIsQuitOpen(false);
    } catch {
      setFormError("Formation Lap could not apply the selected Quit action.");
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
              rememberDialogTrigger();
              setFormError(null);
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
              setFormError(null);
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
              rememberDialogTrigger();
              setFormError(null);
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
            isSaving={isSaving}
            error={formError}
            onChange={(settings) => void updateDesktopSettings(settings)}
            onCheckUpdates={() => void checkUpdates()}
            onOpenDiagnostics={() => void openDiagnostics()}
            onQuit={() => {
              rememberDialogTrigger();
              setIsQuitOpen(true);
            }}
          />
        ) : view === "diagnostics" && state.kind === "ready" ? (
          <DiagnosticsScreen
            diagnostics={diagnosticExport}
            isLoading={isDiagnosticsLoading}
            error={formError}
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
            isSaving={isSaving}
            error={formError}
            onProfileNameChange={setProfileName}
            onPrimarySimNameChange={setPrimarySimName}
            onPrimarySimSourceChange={setPrimarySimSource}
            onSourceValueChange={setSourceValue}
            onPickExecutablePath={pickExecutablePath}
            onSelectPrimarySim={selectDiscoveredPrimarySim}
            onEnterManual={enterManualPrimarySim}
            onToggleSupporting={toggleSupportingApplication}
            onCancel={() => {
              recommendationRequest.current += 1;
              setDiscoveryState({ kind: "idle" });
              setRecommendationState({ kind: "idle" });
              setView("dashboard");
            }}
            onSubmit={createProfile}
          />
        ) : view === "edit-profile" &&
          state.kind === "ready" &&
          profileDraft ? (
          <ProfileEditor
            profile={profileDraft}
            needsReview={selectedProfileNeedsReview}
            isSaving={isSaving}
            error={formError}
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
            error={formError}
            applicationIcons={snapshot?.applicationIcons ?? []}
            applicationProcesses={snapshot?.applicationProcesses ?? []}
            session={snapshot?.session ?? null}
            updates={snapshot?.updates ?? null}
            onlineChecksEnabled={
              snapshot?.settings.automaticUpdateChecks ?? false
            }
            isBusy={isSaving}
            gameLaunchDiagnostic={gameLaunchDiagnostic}
            onCreateProfile={openNewProfile}
            onDeleteProfile={() => {
              rememberDialogTrigger();
              setFormError(null);
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
          {formError && (
            <p className="form-error" role="alert">
              {formError}
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
                disabled={isSaving}
                onClick={() => void requestQuit("leaveApplicationsRunning")}
              >
                Leave applications running
              </button>
            )}
            <button
              type="button"
              className="primary-button"
              disabled={isSaving}
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

      {pendingProcessAction && (
        <ModalDialog
          labelledBy="process-confirmation-title"
          onClose={() => setPendingProcessAction(null)}
        >
          <p className="eyebrow">
            {pendingProcessAction.kind === "force"
              ? "Force termination"
              : "Ownership confirmation"}
          </p>
          <h2 id="process-confirmation-title">
            {pendingProcessAction.kind === "force"
              ? `Force stop ${pendingProcessAction.application.name}?`
              : `${pendingProcessAction.kind === "restart" ? "Restart" : "Control"} a Pre-existing Process?`}
          </h2>
          <p>
            {pendingProcessAction.kind === "force"
              ? `Graceful shutdown did not complete. Force stopping ${pendingProcessAction.application.name} may lose unsaved work.`
              : `${pendingProcessAction.application.name} was already running before Formation Lap observed it. This explicit action will control a Process that the current Session does not own.`}
          </p>
          {formError && (
            <p className="form-error" role="alert">
              {formError}
            </p>
          )}
          <div className="dialog-actions">
            <button
              type="button"
              className="secondary-button"
              onClick={() => setPendingProcessAction(null)}
            >
              Cancel
            </button>
            <button
              type="button"
              className={
                pendingProcessAction.kind === "force"
                  ? "danger-button"
                  : "primary-button"
              }
              disabled={isSaving}
              onClick={confirmProcessAction}
            >
              {pendingProcessAction.kind === "force"
                ? `Force stop ${pendingProcessAction.application.name}`
                : `${pendingProcessAction.kind === "restart" ? "Restart" : "Exit"} ${pendingProcessAction.application.name}`}
            </button>
          </div>
        </ModalDialog>
      )}

      {outputApplication &&
        (() => {
          const processOutput = snapshot?.applicationProcesses.find(
            (process) => process.applicationId === outputApplication.id,
          )?.output;
          return (
            <ModalDialog
              className="console-dialog"
              labelledBy="console-output-title"
              onClose={() => setOutputApplication(null)}
            >
              <p className="eyebrow">Bounded local output</p>
              <h2 id="console-output-title">{outputApplication.name} output</h2>
              <p>
                Formation Lap keeps only the most recent local stdout and stderr
                tail.
              </p>
              <pre className="console-output">
                {processOutput
                  ? [
                      processOutput.stdout,
                      processOutput.stderr,
                      processOutput.truncated
                        ? "\n[Earlier output was discarded.]"
                        : "",
                    ].join("")
                  : "No captured output."}
              </pre>
              <div className="dialog-actions">
                <button
                  type="button"
                  className="primary-button"
                  onClick={() => setOutputApplication(null)}
                >
                  Close output
                </button>
              </div>
            </ModalDialog>
          );
        })()}

      {isDuplicateOpen && selectedProfile && (
        <ModalDialog
          labelledBy="duplicate-profile-title"
          onClose={() => setIsDuplicateOpen(false)}
        >
          <p className="eyebrow">Profile action</p>
          <h2 id="duplicate-profile-title">Duplicate {selectedProfile.name}</h2>
          <p>
            Create an independent copy with the same startup order and settings.
          </p>
          <form onSubmit={duplicateProfile}>
            <label className="field">
              <span>Duplicate name</span>
              <input
                autoFocus
                required
                value={duplicateName}
                onChange={(event) =>
                  setDuplicateName(event.currentTarget.value)
                }
              />
            </label>
            {formError && (
              <p className="form-error" role="alert">
                {formError}
              </p>
            )}
            <div className="dialog-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => setIsDuplicateOpen(false)}
              >
                Cancel
              </button>
              <button
                type="submit"
                className="primary-button"
                disabled={isSaving}
              >
                {isSaving ? "Duplicating…" : "Create duplicate"}
              </button>
            </div>
          </form>
        </ModalDialog>
      )}

      {isDeleteOpen && selectedProfile && (
        <ModalDialog
          labelledBy="delete-profile-title"
          onClose={() => setIsDeleteOpen(false)}
        >
          <p className="eyebrow">Destructive action</p>
          <h2 id="delete-profile-title">Delete {selectedProfile.name}?</h2>
          <p>
            This removes the Racing Profile from your library. Formation Lap
            keeps a bounded local backup for recovery.
          </p>
          {formError && (
            <p className="form-error" role="alert">
              {formError}
            </p>
          )}
          <div className="dialog-actions">
            <button
              type="button"
              className="secondary-button"
              onClick={() => setIsDeleteOpen(false)}
            >
              Cancel
            </button>
            <button
              type="button"
              className="danger-button"
              disabled={isSaving}
              onClick={() => void deleteProfile()}
            >
              {isSaving ? "Deleting…" : `Delete ${selectedProfile.name}`}
            </button>
          </div>
        </ModalDialog>
      )}

      {isExportOpen && selectedProfile && (
        <ModalDialog
          className="transfer-dialog"
          labelledBy="export-profile-title"
          onClose={() => setIsExportOpen(false)}
        >
          <p className="eyebrow">Portable profile</p>
          <h2 id="export-profile-title">Export {selectedProfile.name}</h2>
          <p>
            Copy this JSON into a local text file. Machine-specific paths are
            marked for repair when imported elsewhere.
          </p>
          <label className="field">
            <span>Portable profile JSON</span>
            <textarea
              readOnly
              rows={12}
              value={exportDocument}
              aria-busy={isSaving}
            />
          </label>
          {formError && (
            <p className="form-error" role="alert">
              {formError}
            </p>
          )}
          <div className="dialog-actions">
            <button
              type="button"
              className="primary-button"
              onClick={() => setIsExportOpen(false)}
            >
              Close export
            </button>
          </div>
        </ModalDialog>
      )}

      {isImportOpen && (
        <ModalDialog
          className="transfer-dialog"
          labelledBy="import-profile-title"
          onClose={() => setIsImportOpen(false)}
        >
          <p className="eyebrow">Portable profile</p>
          <h2 id="import-profile-title">Import Racing Profile</h2>
          <p>
            Paste a Formation Lap profile document. A fresh local identity is
            assigned and missing paths remain visible for repair.
          </p>
          <form onSubmit={importProfile}>
            <label className="field">
              <span>Portable profile JSON</span>
              <textarea
                autoFocus
                required
                rows={12}
                value={importDocument}
                onChange={(event) =>
                  setImportDocument(event.currentTarget.value)
                }
              />
            </label>
            {formError && (
              <p className="form-error" role="alert">
                {formError}
              </p>
            )}
            <div className="dialog-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => setIsImportOpen(false)}
              >
                Cancel
              </button>
              <button
                type="submit"
                className="primary-button"
                disabled={isSaving}
              >
                {isSaving ? "Importing…" : "Import Racing Profile"}
              </button>
            </div>
          </form>
        </ModalDialog>
      )}
    </div>
  );
}
