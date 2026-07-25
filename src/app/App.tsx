import {
  useEffect,
  useRef,
  useState,
  type FormEvent,
  type ReactNode,
} from "react";
import markUrl from "../assets/formation-lap-mark.svg";
import type {
  ApplicationUpdateSnapshot,
  ApplicationProcessSnapshot,
  AppSnapshot,
  DesktopSettings,
  DiagnosticExport,
  DiscoveredInstallation,
  DiscoveredPrimarySim,
  DiscoveredSupportingApplication,
  DiscoverySnapshot,
  GameLaunchDiagnostic,
  LaunchSource,
  ProfileApplication,
  RacingProfile,
  QuitDisposition,
  SessionApplicationSnapshot,
  SupportingApplication,
  SupportingApplicationRecommendation,
  UpdateSnapshot,
} from "../generated/bindings";
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

type DiscoveryState =
  | { kind: "idle" }
  | { kind: "loading" }
  | { kind: "ready"; snapshot: DiscoverySnapshot }
  | { kind: "error" };

type RecommendationState =
  | { kind: "idle" }
  | { kind: "loading"; primarySimName: string }
  | {
      kind: "ready";
      primarySimName: string;
      recommendations: SupportingApplicationRecommendation[];
    }
  | { kind: "error"; primarySimName: string };

type WorkspaceView =
  "dashboard" | "new-profile" | "edit-profile" | "settings" | "diagnostics";
type PrimarySimSource = "direct" | "steam";
type PendingProcessAction = {
  kind: "exit" | "restart" | "force";
  application: ProfileApplication;
  process: ApplicationProcessSnapshot;
};
type ProfileApproval = {
  configurationReviewed: boolean;
  approvedPrivilegedApplicationIds: string[];
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

interface ModalDialogProps {
  children: ReactNode;
  className?: string;
  labelledBy: string;
  onClose(): void;
}

function ModalDialog({
  children,
  className,
  labelledBy,
  onClose,
}: ModalDialogProps) {
  const dialog = useRef<HTMLDialogElement | null>(null);

  useEffect(() => {
    const element = dialog.current;
    if (!element) {
      return;
    }

    try {
      element.showModal();
    } catch {
      element.setAttribute("open", "");
    }

    return () => {
      if (element.open && typeof element.close === "function") {
        element.close();
      }
    };
  }, []);

  return (
    <dialog
      ref={dialog}
      className={`profile-dialog ${className ?? ""}`}
      aria-labelledby={labelledBy}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          onClose();
        }
      }}
    >
      {children}
    </dialog>
  );
}

interface SettingsScreenProps {
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

function updateStatusLabel(status: UpdateSnapshot["formationLap"]): string {
  switch (status.kind) {
    case "current":
      return `Current · ${status.currentVersion}`;
    case "updateAvailable":
      return `Update available · ${status.latestVersion}`;
    case "unknown":
      return "Unknown";
  }
}

function SettingsScreen({
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

interface DiagnosticsScreenProps {
  diagnostics: DiagnosticExport | null;
  isLoading: boolean;
  error: string | null;
  onRefresh(): void;
}

function DiagnosticsScreen({
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

function launchSourceFromInstallation(
  installation: DiscoveredInstallation,
): LaunchSource {
  return installation.kind === "steam"
    ? { kind: "steam", appId: installation.appId, selector: null }
    : {
        kind: "directExecutable",
        executablePath: installation.executablePath,
      };
}

function installationWorkingDirectory(
  installation: DiscoveredInstallation,
): string | null {
  if (installation.kind === "steam") {
    return installation.install_directory;
  }

  return directoryFromPath(installation.executablePath);
}

function directoryFromPath(path: string): string | null {
  const lastSeparator = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  return lastSeparator > 0 ? path.slice(0, lastSeparator) : null;
}

function displayWindowsPath(path: string): string {
  const extendedUncPrefix = "\\\\?\\UNC\\";
  const extendedPathPrefix = "\\\\?\\";
  if (path.startsWith(extendedUncPrefix)) {
    return `\\\\${path.slice(extendedUncPrefix.length)}`;
  }
  return path.startsWith(extendedPathPrefix)
    ? path.slice(extendedPathPrefix.length)
    : path;
}

function commandErrorMessage(error: unknown, fallback: string): string {
  const details =
    typeof error === "string"
      ? (() => {
          try {
            return JSON.parse(error) as unknown;
          } catch {
            return null;
          }
        })()
      : error;
  if (typeof details !== "object" || details === null) {
    return fallback;
  }
  const { message, recovery } = details as {
    message?: unknown;
    recovery?: unknown;
  };
  if (typeof message !== "string" || message.trim().length === 0) {
    return fallback;
  }
  return typeof recovery === "string" && recovery.trim().length > 0
    ? `${message} ${recovery}`
    : message;
}

function executableNameFromPath(path: string): string | null {
  const lastSeparator = Math.max(path.lastIndexOf("\\"), path.lastIndexOf("/"));
  const fileName = path.slice(lastSeparator + 1);
  return fileName.length > 0 ? fileName : null;
}

function discoveredSupportingApplicationToProfile(
  application: DiscoveredSupportingApplication,
): SupportingApplication {
  return {
    application: {
      id: crypto.randomUUID(),
      name: application.name,
      launchRecipe: {
        source: launchSourceFromInstallation(application.installation),
        arguments: [],
        workingDirectory: installationWorkingDirectory(
          application.installation,
        ),
        monitoredProcess: null,
        monitoredExecutablePath: null,
        consoleVisibility: "hidden",
        elevated: false,
        startupTimeoutSeconds: 30,
        postStartDelayMilliseconds: 0,
        shutdownStrategy: { kind: "closeWindows" },
      },
      pathNeedsRepair: false,
    },
    requirement: "optional",
    keepRunning: false,
  };
}

function installationSourceLabel(installation: DiscoveredInstallation): string {
  return installation.kind === "steam" ? "Steam" : "Standalone";
}

function applicationIcon(
  application: DiscoveredPrimarySim | DiscoveredSupportingApplication,
) {
  return application.icon.kind === "localData" ? (
    <img
      alt=""
      src={`data:${application.icon.media_type};base64,${application.icon.data_base64}`}
    />
  ) : (
    <FlagIcon />
  );
}

function profileApplicationIcon(
  applicationId: string,
  applicationIcons: NonNullable<AppSnapshot["applicationIcons"]>,
  fallback: ReactNode,
) {
  const icon = applicationIcons.find(
    (candidate) => candidate.applicationId === applicationId,
  )?.icon;
  return icon?.kind === "localData" ? (
    <img alt="" src={`data:${icon.media_type};base64,${icon.data_base64}`} />
  ) : (
    fallback
  );
}

interface ProfileWizardProps {
  profileName: string;
  primarySimName: string;
  primarySimSource: PrimarySimSource;
  sourceValue: string;
  discoveryState: DiscoveryState;
  recommendationState: RecommendationState;
  selectedPrimarySimId: string | null;
  selectedSupportingIds: string[];
  isManualEntry: boolean;
  isSaving: boolean;
  error: string | null;
  onProfileNameChange(value: string): void;
  onPrimarySimNameChange(value: string): void;
  onPrimarySimSourceChange(value: PrimarySimSource): void;
  onSourceValueChange(value: string): void;
  onPickExecutablePath(initialPath?: string | null): Promise<string | null>;
  onSelectPrimarySim(primarySim: DiscoveredPrimarySim): void;
  onEnterManual(): void;
  onToggleSupporting(applicationId: string): void;
  onCancel(): void;
  onSubmit(event: FormEvent<HTMLFormElement>): void;
}

function ProfileWizard({
  profileName,
  primarySimName,
  primarySimSource,
  sourceValue,
  discoveryState,
  recommendationState,
  selectedPrimarySimId,
  selectedSupportingIds,
  isManualEntry,
  isSaving,
  error,
  onProfileNameChange,
  onPrimarySimNameChange,
  onPrimarySimSourceChange,
  onSourceValueChange,
  onPickExecutablePath,
  onSelectPrimarySim,
  onEnterManual,
  onToggleSupporting,
  onCancel,
  onSubmit,
}: ProfileWizardProps) {
  const selectedSupportingApplications =
    discoveryState.kind === "ready"
      ? discoveryState.snapshot.installedSupportingApplications.filter(
          (application) => selectedSupportingIds.includes(application.id),
        )
      : [];

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
              <h2 id="sim-step-title">Choose a Primary Sim</h2>
              <p>
                Formation Lap found these Curated Catalog sims on this PC. The
                Primary Sim always launches last.
              </p>

              {discoveryState.kind === "loading" && (
                <div className="discovery-status" role="status">
                  <PulseIcon />
                  <span>
                    <strong>Checking known locations</strong>
                    <small>
                      Only targeted Steam and Windows locations are inspected.
                    </small>
                  </span>
                </div>
              )}

              {discoveryState.kind === "error" && (
                <div className="discovery-status discovery-status-warning">
                  <FlagIcon />
                  <span>
                    <strong>Automatic discovery is unavailable</strong>
                    <small>
                      You can still configure the Primary Sim manually.
                    </small>
                  </span>
                </div>
              )}

              {discoveryState.kind === "ready" &&
                discoveryState.snapshot.installedPrimarySims.length > 0 && (
                  <div className="sim-picker">
                    {discoveryState.snapshot.installedPrimarySims.map(
                      (primarySim) => {
                        const isSelected =
                          primarySim.id === selectedPrimarySimId;
                        const sourceLabel = installationSourceLabel(
                          primarySim.installation,
                        );
                        return (
                          <button
                            type="button"
                            className="sim-choice"
                            aria-label={`Use ${primarySim.name} (${sourceLabel})`}
                            aria-pressed={isSelected}
                            key={`${primarySim.id}:${sourceLabel}`}
                            onClick={() => onSelectPrimarySim(primarySim)}
                          >
                            <span className="sim-choice-icon">
                              {applicationIcon(primarySim)}
                            </span>
                            <span className="sim-choice-copy">
                              <strong>{primarySim.name}</strong>
                              <small>{sourceLabel}</small>
                            </span>
                            <span className="sim-choice-status">
                              <CheckIcon />
                              Installed
                            </span>
                          </button>
                        );
                      },
                    )}
                  </div>
                )}

              {discoveryState.kind === "ready" &&
                discoveryState.snapshot.installedPrimarySims.length === 0 && (
                  <div className="discovery-status">
                    <FlagIcon />
                    <span>
                      <strong>No Curated Catalog sim was found</strong>
                      <small>
                        Manual Entry keeps uncommon and custom installs
                        available.
                      </small>
                    </span>
                  </div>
                )}

              <div className="manual-entry-action">
                <span>Not listed or using a custom executable?</span>
                <button
                  type="button"
                  className="text-button"
                  onClick={onEnterManual}
                >
                  Enter a sim manually
                </button>
              </div>

              {isManualEntry && (
                <div className="manual-entry-panel">
                  <p className="eyebrow">Manual Entry</p>
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
                    <div className="path-input">
                      <input
                        aria-label={
                          primarySimSource === "steam"
                            ? "Steam App ID"
                            : "Executable path"
                        }
                        inputMode={
                          primarySimSource === "steam" ? "numeric" : "text"
                        }
                        value={
                          primarySimSource === "direct"
                            ? displayWindowsPath(sourceValue)
                            : sourceValue
                        }
                        onChange={(event) =>
                          onSourceValueChange(event.currentTarget.value)
                        }
                        placeholder={
                          primarySimSource === "steam"
                            ? "2399420"
                            : String.raw`C:\Games\Le Mans Ultimate\LMU.exe`
                        }
                      />
                      {primarySimSource === "direct" && (
                        <button
                          type="button"
                          className="secondary-button path-browse-button"
                          aria-label="Browse for Primary Sim executable"
                          onClick={() =>
                            void onPickExecutablePath(sourceValue).then(
                              (path) => {
                                if (path) {
                                  onSourceValueChange(path);
                                }
                              },
                            )
                          }
                        >
                          Browseâ€¦
                        </button>
                      )}
                    </div>
                    <small>
                      You can leave this blank and repair the path in the
                      profile editor.
                    </small>
                  </label>
                </div>
              )}

              {!isManualEntry && selectedPrimarySimId && (
                <div className="selected-sim-summary" role="status">
                  <CheckIcon />
                  <span>
                    <strong>{primarySimName} selected</strong>
                    <small>
                      {primarySimSource === "steam"
                        ? `Steam App ID ${sourceValue}`
                        : displayWindowsPath(sourceValue)}
                    </small>
                  </span>
                </div>
              )}
            </div>
          </section>

          {!isManualEntry && selectedPrimarySimId && (
            <section
              className="wizard-step"
              aria-labelledby="recommendations-step-title"
            >
              <span className="step-index">03</span>
              <div>
                <h2 id="recommendations-step-title">
                  Add Supporting Applications
                </h2>
                <p>
                  Installed recommendations are optional and launch before the
                  Primary Sim.
                </p>
                <div
                  className="recommendation-region"
                  role="region"
                  aria-label={`Recommended for ${primarySimName}`}
                >
                  {recommendationState.kind === "loading" && (
                    <div className="recommendation-empty" role="status">
                      <PulseIcon />
                      <span>Ranking installed applications…</span>
                    </div>
                  )}
                  {recommendationState.kind === "error" && (
                    <div className="recommendation-empty">
                      <FlagIcon />
                      <span>Recommendations could not be loaded.</span>
                    </div>
                  )}
                  {recommendationState.kind === "ready" &&
                    recommendationState.recommendations.length === 0 && (
                      <div className="recommendation-empty">
                        <CheckIcon />
                        <span>No installed recommendations to add.</span>
                      </div>
                    )}
                  {recommendationState.kind === "ready" &&
                    recommendationState.recommendations.length > 0 && (
                      <div className="recommendation-list">
                        {recommendationState.recommendations.map(
                          (recommendation) => {
                            const installedApplication =
                              discoveryState.kind === "ready"
                                ? discoveryState.snapshot.installedSupportingApplications.find(
                                    (application) =>
                                      application.id === recommendation.id,
                                  )
                                : undefined;
                            const rankLabel =
                              recommendation.rank === "recommended"
                                ? "Recommended"
                                : "Compatible";
                            return (
                              <label
                                className="recommendation-row"
                                key={recommendation.id}
                              >
                                <input
                                  type="checkbox"
                                  aria-label={`Add ${recommendation.name}`}
                                  checked={selectedSupportingIds.includes(
                                    recommendation.id,
                                  )}
                                  onChange={() =>
                                    onToggleSupporting(recommendation.id)
                                  }
                                />
                                <span className="recommendation-icon">
                                  {installedApplication ? (
                                    applicationIcon(installedApplication)
                                  ) : (
                                    <FlagIcon />
                                  )}
                                </span>
                                <span className="recommendation-copy">
                                  <strong>{recommendation.name}</strong>
                                  <small>
                                    {recommendation.updateProvider?.kind ===
                                    "githubReleases"
                                      ? "Update notifications via GitHub Releases"
                                      : "Detected on this PC"}
                                  </small>
                                </span>
                                <span
                                  className={`recommendation-rank recommendation-rank-${recommendation.rank}`}
                                >
                                  {rankLabel}
                                </span>
                              </label>
                            );
                          },
                        )}
                      </div>
                    )}
                </div>
              </div>
            </section>
          )}

          {error && (
            <p className="form-error" role="alert">
              {error}
            </p>
          )}

          <div className="wizard-actions">
            <button
              type="submit"
              className="primary-button"
              disabled={isSaving || primarySimName.trim().length === 0}
            >
              {isSaving ? "Creating profile…" : "Create Racing Profile"}
            </button>
          </div>
        </div>

        <aside className="wizard-preview" aria-labelledby="preview-title">
          <div className="preview-heading">
            <span className="step-index">04</span>
            <div>
              <p className="eyebrow">Review</p>
              <h2 id="preview-title">Startup order</h2>
            </div>
          </div>
          <div className="order-preview">
            {selectedSupportingApplications.length > 0 ? (
              selectedSupportingApplications.map((application) => (
                <div className="order-application-row" key={application.id}>
                  <span className="application-icon">
                    {applicationIcon(application)}
                  </span>
                  <span>
                    <strong>{application.name}</strong>
                    <small>Supporting Application</small>
                  </span>
                </div>
              ))
            ) : (
              <div className="order-empty">
                <PlusIcon />
                <span>
                  <strong>Supporting Applications</strong>
                  <small>Select installed recommendations to add them.</small>
                </span>
              </div>
            )}
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
                    ? "Steam"
                    : "Standalone / Direct executable"}
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

interface ProfileEditorProps {
  profile: RacingProfile;
  needsReview: boolean;
  isSaving: boolean;
  error: string | null;
  onPickExecutablePath(initialPath?: string | null): Promise<string | null>;
  onChange(profile: RacingProfile): void;
  onCancel(): void;
  onSubmit(event: FormEvent<HTMLFormElement>, approval?: ProfileApproval): void;
}

function ProfileEditor({
  profile,
  needsReview,
  isSaving,
  error,
  onPickExecutablePath,
  onChange,
  onCancel,
  onSubmit,
}: ProfileEditorProps) {
  const [configurationReviewed, setConfigurationReviewed] = useState(false);
  const [
    approvedPrivilegedApplicationIds,
    setApprovedPrivilegedApplicationIds,
  ] = useState<string[]>([]);
  const profileApplications = [
    profile.primarySim,
    ...profile.supportingApplications.map(
      (supporting) => supporting.application,
    ),
  ];
  const privilegedApplications = profileApplications.filter(
    (application) =>
      application.launchRecipe.elevated ||
      application.launchRecipe.shutdownStrategy.kind === "customStop",
  );
  const pathsNeedRepair = profileApplications.some(
    (application) => application.pathNeedsRepair,
  );
  const applicationsNeedingRepair = profileApplications.filter(
    (application) => application.pathNeedsRepair,
  );
  const approvalComplete =
    configurationReviewed &&
    privilegedApplications.every((application) =>
      approvedPrivilegedApplicationIds.includes(application.id),
    );

  const update = (change: (next: RacingProfile) => void) => {
    const next = structuredClone(profile);
    change(next);
    onChange(next);
  };

  const updateSupportingApplication = (
    index: number,
    change: (supporting: SupportingApplication) => void,
  ) => {
    update((next) => {
      const supportingApplication = next.supportingApplications[index];
      if (supportingApplication) {
        change(supportingApplication);
      }
    });
  };

  const addSupportingApplication = () => {
    update((next) => {
      next.supportingApplications.push({
        application: {
          id: crypto.randomUUID(),
          name: "New Supporting Application",
          launchRecipe: {
            source: { kind: "directExecutable", executablePath: "" },
            arguments: [],
            workingDirectory: null,
            monitoredProcess: null,
            monitoredExecutablePath: null,
            consoleVisibility: "hidden",
            elevated: false,
            startupTimeoutSeconds: 30,
            postStartDelayMilliseconds: 0,
            shutdownStrategy: { kind: "closeWindows" },
          },
          pathNeedsRepair: true,
        },
        requirement: "optional",
        keepRunning: false,
      });
    });
  };

  const moveSupportingApplication = (index: number, direction: -1 | 1) => {
    update((next) => {
      const destination = index + direction;
      if (
        destination < 0 ||
        destination >= next.supportingApplications.length
      ) {
        return;
      }
      const [application] = next.supportingApplications.splice(index, 1);
      if (application) {
        next.supportingApplications.splice(destination, 0, application);
      }
    });
  };

  return (
    <form
      className="profile-editor"
      onSubmit={(event) =>
        onSubmit(
          event,
          needsReview
            ? {
                configurationReviewed,
                approvedPrivilegedApplicationIds,
              }
            : undefined,
        )
      }
    >
      <header className="workspace-header editor-header">
        <div>
          <p className="eyebrow">Profile editor</p>
          <h1>{profile.name}</h1>
          <p className="workspace-summary">
            Configure launch behavior on the left and keep the Primary Sim
            locked last on the right.
          </p>
        </div>
        <div className="editor-header-actions">
          <div className="header-actions">
            <button
              type="button"
              className="secondary-button"
              onClick={onCancel}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="primary-button"
              disabled={isSaving || (needsReview && !approvalComplete)}
            >
              {isSaving
                ? "Saving…"
                : needsReview
                  ? "Save and approve profile"
                  : "Save changes"}
            </button>
          </div>
          {error && (
            <p className="form-error editor-header-error" role="alert">
              {error}
            </p>
          )}
        </div>
      </header>

      <div className="profile-editor-grid">
        <div className="editor-column">
          {needsReview && (
            <section
              className="editor-panel profile-review-panel"
              aria-labelledby="profile-review-editor-title"
            >
              <div className="editor-panel-heading">
                <p className="eyebrow">Native launch quarantine</p>
                <h2 id="profile-review-editor-title">
                  Approve reviewed configuration
                </h2>
              </div>
              <p>
                Confirm the preserved portable values below. Missing or
                suspicious paths must be repaired before approval.
              </p>
              <label className="check-row">
                <input
                  type="checkbox"
                  checked={configurationReviewed}
                  onChange={(event) =>
                    setConfigurationReviewed(event.currentTarget.checked)
                  }
                />
                <span>
                  <strong>
                    I reviewed executable paths, arguments, working directories,
                    elevation, monitored executables, and stop recipes.
                  </strong>
                </span>
              </label>
              {privilegedApplications.map((application) => (
                <label className="check-row" key={application.id}>
                  <input
                    type="checkbox"
                    checked={approvedPrivilegedApplicationIds.includes(
                      application.id,
                    )}
                    onChange={(event) => {
                      const isApproved = event.currentTarget.checked;
                      setApprovedPrivilegedApplicationIds((current) =>
                        isApproved
                          ? [...current, application.id]
                          : current.filter(
                              (applicationId) =>
                                applicationId !== application.id,
                            ),
                      );
                    }}
                  />
                  <span>
                    <strong>
                      Approve privileged recipe for {application.name}
                    </strong>
                    <small>
                      {application.launchRecipe.elevated
                        ? "Elevated launch"
                        : "Ordinary launch"}
                      {application.launchRecipe.shutdownStrategy.kind ===
                      "customStop"
                        ? " · custom stop executable"
                        : ""}
                    </small>
                  </span>
                </label>
              ))}
              {pathsNeedRepair && (
                <div className="profile-repair-list" role="alert">
                  <strong>Select an executable for:</strong>
                  <ul>
                    {applicationsNeedingRepair.map((application) => (
                      <li key={application.id}>
                        {application.name} needs an executable path.
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </section>
          )}
          <section className="editor-panel" aria-labelledby="identity-title">
            <div className="editor-panel-heading">
              <p className="eyebrow">Profile</p>
              <h2 id="identity-title">Launch and Session behavior</h2>
            </div>
            <label className="field">
              <span>Profile name</span>
              <input
                required
                value={profile.name}
                onChange={(event) =>
                  update((next) => {
                    next.name = event.currentTarget.value;
                  })
                }
              />
            </label>
            <div className="settings-checks">
              <label className="check-row">
                <input
                  type="checkbox"
                  aria-label="VR enabled by default"
                  checked={profile.vrEnabled}
                  onChange={(event) =>
                    update((next) => {
                      next.vrEnabled = event.currentTarget.checked;
                    })
                  }
                />
                <span>
                  <strong>VR enabled by default</strong>
                  <small>Remember this choice on the Dashboard.</small>
                </span>
              </label>
              <label className="field compact-field">
                <span>Preferred VR Launch Mode</span>
                <select
                  value={profile.preferredVrLaunchMode ?? ""}
                  onChange={(event) =>
                    update((next) => {
                      const value = event.currentTarget.value;
                      next.preferredVrLaunchMode =
                        value === ""
                          ? null
                          : (value as NonNullable<
                              RacingProfile["preferredVrLaunchMode"]
                            >);
                    })
                  }
                >
                  <option value="">Use ordinary recipe</option>
                  <option value="openXr">OpenXR</option>
                  <option value="openVr">OpenVR / SteamVR</option>
                  <option value="oculus">Oculus</option>
                </select>
              </label>
              <label className="check-row">
                <input
                  type="checkbox"
                  checked={profile.closeSession.stopSteamVr}
                  onChange={(event) =>
                    update((next) => {
                      next.closeSession.stopSteamVr =
                        event.currentTarget.checked;
                    })
                  }
                />
                <span>
                  <strong>Stop SteamVR on Close Session</strong>
                  <small>Only when this Session started SteamVR.</small>
                </span>
              </label>
            </div>
          </section>

          <section className="editor-panel" aria-labelledby="primary-sim-title">
            <div className="editor-panel-heading">
              <p className="eyebrow">Primary Sim</p>
              <h2 id="primary-sim-title">Game Launch Recipe</h2>
            </div>
            <label className="field">
              <span>Primary Sim name</span>
              <input
                required
                value={profile.primarySim.name}
                onChange={(event) =>
                  update((next) => {
                    next.primarySim.name = event.currentTarget.value;
                  })
                }
              />
            </label>
            <ApplicationRecipeFields
              application={profile.primarySim}
              label="Primary Sim"
              onPickExecutablePath={onPickExecutablePath}
              onChange={(application) =>
                update((next) => {
                  next.primarySim = application;
                })
              }
            />
          </section>
        </div>

        <section
          className="editor-panel startup-editor"
          aria-labelledby="order-title"
        >
          <div className="editor-panel-heading order-heading">
            <div>
              <p className="eyebrow">Startup order</p>
              <h2 id="order-title">Supporting Applications</h2>
            </div>
            <button
              type="button"
              className="secondary-button"
              onClick={addSupportingApplication}
            >
              <PlusIcon />
              Add application
            </button>
          </div>

          <div className="supporting-editor-list">
            {profile.supportingApplications.length === 0 && (
              <div className="application-empty">
                <PlusIcon />
                <span>
                  <strong>No Supporting Applications</strong>
                  <small>Add them in the order they should start.</small>
                </span>
              </div>
            )}
            {profile.supportingApplications.map(
              (supportingApplication, index) => (
                <article
                  className="supporting-editor-row"
                  key={supportingApplication.application.id}
                >
                  <div className="supporting-row-heading">
                    <span className="drag-order" aria-hidden="true">
                      {String(index + 1).padStart(2, "0")}
                    </span>
                    <label className="field inline-name-field">
                      <span>Supporting Application {index + 1} name</span>
                      <input
                        required
                        value={supportingApplication.application.name}
                        onChange={(event) =>
                          updateSupportingApplication(index, (supporting) => {
                            supporting.application.name =
                              event.currentTarget.value;
                          })
                        }
                      />
                    </label>
                    <div className="row-actions">
                      <button
                        type="button"
                        className="tertiary-button"
                        aria-label={`Move ${supportingApplication.application.name} up`}
                        disabled={index === 0}
                        onClick={() => moveSupportingApplication(index, -1)}
                      >
                        ↑
                      </button>
                      <button
                        type="button"
                        className="tertiary-button"
                        aria-label={`Move ${supportingApplication.application.name} down`}
                        disabled={
                          index === profile.supportingApplications.length - 1
                        }
                        onClick={() => moveSupportingApplication(index, 1)}
                      >
                        ↓
                      </button>
                      <button
                        type="button"
                        className="tertiary-button danger-text"
                        aria-label={`Remove ${supportingApplication.application.name}`}
                        onClick={() =>
                          update((next) => {
                            next.supportingApplications.splice(index, 1);
                          })
                        }
                      >
                        Remove
                      </button>
                    </div>
                  </div>

                  <div className="supporting-policy">
                    <label className="field compact-field">
                      <span>
                        Requirement for {supportingApplication.application.name}
                      </span>
                      <select
                        value={supportingApplication.requirement}
                        onChange={(event) =>
                          updateSupportingApplication(index, (supporting) => {
                            supporting.requirement = event.currentTarget
                              .value as SupportingApplication["requirement"];
                          })
                        }
                      >
                        <option value="required">Required</option>
                        <option value="optional">Optional</option>
                      </select>
                    </label>
                    <label className="check-row compact-check">
                      <input
                        type="checkbox"
                        checked={supportingApplication.keepRunning}
                        onChange={(event) =>
                          updateSupportingApplication(index, (supporting) => {
                            supporting.keepRunning =
                              event.currentTarget.checked;
                          })
                        }
                      />
                      <span>
                        <strong>
                          Keep {supportingApplication.application.name} running
                        </strong>
                        <small>Detach it after Close Session.</small>
                      </span>
                    </label>
                  </div>

                  <ApplicationRecipeFields
                    application={supportingApplication.application}
                    label={supportingApplication.application.name}
                    onPickExecutablePath={onPickExecutablePath}
                    onChange={(application) =>
                      updateSupportingApplication(index, (supporting) => {
                        supporting.application = application;
                      })
                    }
                  />
                </article>
              ),
            )}
          </div>

          <div className="game-divider">
            <span />
            <small>Primary Sim · locked last</small>
          </div>
          <div className="application-row game-row locked-game-row">
            <span className="application-icon game-icon">
              <FlagIcon />
            </span>
            <span className="application-copy">
              <strong>{profile.primarySim.name}</strong>
              <small>Always launches after Supporting Applications</small>
            </span>
            <span className="locked-label">Locked</span>
          </div>
        </section>
      </div>
    </form>
  );
}

interface ApplicationRecipeFieldsProps {
  application: ProfileApplication;
  label: string;
  onPickExecutablePath(initialPath?: string | null): Promise<string | null>;
  onChange(application: ProfileApplication): void;
}

function ApplicationRecipeFields({
  application,
  label,
  onPickExecutablePath,
  onChange,
}: ApplicationRecipeFieldsProps) {
  const update = (change: (next: ProfileApplication) => void) => {
    const next = structuredClone(application);
    change(next);
    onChange(next);
  };
  const source = application.launchRecipe.source;
  const shutdown = application.launchRecipe.shutdownStrategy;
  const setDirectExecutable = (next: ProfileApplication, path: string) => {
    const nextSource = next.launchRecipe.source;
    if (nextSource.kind !== "directExecutable") {
      return;
    }
    nextSource.executablePath = path;
    next.pathNeedsRepair = path.length === 0;
    next.launchRecipe.workingDirectory = directoryFromPath(path);
    next.launchRecipe.monitoredProcess = executableNameFromPath(path);
    next.launchRecipe.monitoredExecutablePath = path || null;
  };
  const selectExecutable = async (
    initialPath: string | null,
    change: (next: ProfileApplication, path: string) => void,
  ) => {
    const path = await onPickExecutablePath(initialPath);
    if (path) {
      update((next) => change(next, path));
    }
  };

  return (
    <details className="recipe-details">
      <summary>Launch Recipe details</summary>
      <div className="recipe-fields">
        <div className="field-grid">
          <label className="field">
            <span>{label} source</span>
            <select
              value={source.kind}
              onChange={(event) =>
                update((next) => {
                  next.launchRecipe.source =
                    event.currentTarget.value === "steam"
                      ? { kind: "steam", appId: 0, selector: null }
                      : { kind: "directExecutable", executablePath: "" };
                  next.pathNeedsRepair =
                    next.launchRecipe.source.kind === "directExecutable";
                })
              }
            >
              <option value="directExecutable">Direct executable</option>
              <option value="steam">Steam</option>
            </select>
          </label>
          <label className="field">
            <span>
              {source.kind === "steam"
                ? `${label} Steam App ID`
                : `${label} executable path`}
            </span>
            <div className="path-input">
              <input
                value={
                  source.kind === "steam"
                    ? String(source.appId || "")
                    : displayWindowsPath(source.executablePath)
                }
                onChange={(event) =>
                  update((next) => {
                    const nextSource = next.launchRecipe.source;
                    if (nextSource.kind === "steam") {
                      nextSource.appId =
                        Number.parseInt(event.currentTarget.value, 10) || 0;
                      next.pathNeedsRepair = false;
                    } else {
                      setDirectExecutable(next, event.currentTarget.value);
                    }
                  })
                }
              />
              {source.kind === "directExecutable" && (
                <button
                  type="button"
                  className="secondary-button path-browse-button"
                  aria-label={`Browse for ${label} executable`}
                  onClick={() =>
                    void selectExecutable(
                      source.kind === "directExecutable"
                        ? source.executablePath
                        : null,
                      (next, path) => {
                        const nextSource = next.launchRecipe.source;
                        if (nextSource.kind !== "directExecutable") {
                          return;
                        }
                        setDirectExecutable(next, path);
                      },
                    )
                  }
                >
                  Browseâ€¦
                </button>
              )}
            </div>
          </label>
        </div>
        {source.kind === "steam" && (
          <div className="field-grid">
            <label className="field">
              <span>{label} Steam launch option</span>
              <select
                value={source.selector?.kind ?? "curated"}
                onChange={(event) =>
                  update((next) => {
                    const nextSource = next.launchRecipe.source;
                    if (nextSource.kind !== "steam") {
                      return;
                    }
                    switch (event.currentTarget.value) {
                      case "default":
                        nextSource.selector = { kind: "default" };
                        break;
                      case "openVr":
                        nextSource.selector = { kind: "openVr" };
                        break;
                      case "oculus":
                        nextSource.selector = { kind: "oculus" };
                        break;
                      case "option":
                        nextSource.selector = { kind: "option", index: 1 };
                        break;
                      default:
                        nextSource.selector = null;
                    }
                  })
                }
              >
                <option value="curated">Use Curated Catalog</option>
                <option value="default">Default</option>
                <option value="openVr">OpenVR / SteamVR</option>
                <option value="oculus">Oculus</option>
                <option value="option">Numbered launch option</option>
              </select>
            </label>
            {source.selector?.kind === "option" && (
              <label className="field">
                <span>Launch option index</span>
                <input
                  type="number"
                  min="0"
                  max="255"
                  value={source.selector.index}
                  onChange={(event) =>
                    update((next) => {
                      const nextSource = next.launchRecipe.source;
                      if (
                        nextSource.kind === "steam" &&
                        nextSource.selector?.kind === "option"
                      ) {
                        nextSource.selector.index = Math.min(
                          255,
                          Math.max(0, event.currentTarget.valueAsNumber || 0),
                        );
                      }
                    })
                  }
                />
              </label>
            )}
          </div>
        )}
        <label className="field">
          <span>{label} arguments · one per line</span>
          <textarea
            rows={2}
            value={application.launchRecipe.arguments.join("\n")}
            onChange={(event) =>
              update((next) => {
                next.launchRecipe.arguments = event.currentTarget.value
                  .split("\n")
                  .filter((argument) => argument.length > 0);
              })
            }
          />
        </label>
        {source.kind === "steam" ? (
          <>
            <div className="field-grid">
              <label className="field">
                <span>{label} working directory</span>
                <input
                  value={application.launchRecipe.workingDirectory ?? ""}
                  onChange={(event) =>
                    update((next) => {
                      next.launchRecipe.workingDirectory =
                        event.currentTarget.value || null;
                    })
                  }
                />
              </label>
              <label className="field">
                <span>{label} monitored process</span>
                <input
                  value={application.launchRecipe.monitoredProcess ?? ""}
                  onChange={(event) =>
                    update((next) => {
                      next.launchRecipe.monitoredProcess =
                        event.currentTarget.value || null;
                      next.launchRecipe.monitoredExecutablePath = null;
                    })
                  }
                />
              </label>
            </div>
            <label className="field">
              <span>{label} monitored executable path</span>
              <div className="path-input">
                <input
                  value={displayWindowsPath(
                    application.launchRecipe.monitoredExecutablePath ?? "",
                  )}
                  onChange={(event) =>
                    update((next) => {
                      next.launchRecipe.monitoredExecutablePath =
                        event.currentTarget.value || null;
                    })
                  }
                />
                <button
                  type="button"
                  className="secondary-button path-browse-button"
                  aria-label={`Browse for ${label} monitored executable`}
                  onClick={() =>
                    void selectExecutable(
                      application.launchRecipe.monitoredExecutablePath ?? null,
                      (next, path) => {
                        next.launchRecipe.monitoredExecutablePath = path;
                      },
                    )
                  }
                >
                  Browseâ€¦
                </button>
              </div>
              <small>
                Required before a launcher-discovered process can be
                Session-owned. Test Game Launch can learn this path for review.
              </small>
            </label>
          </>
        ) : (
          <p className="recipe-derived-details">
            Formation Lap will launch from this executable&apos;s folder and
            monitor this exact executable automatically.
          </p>
        )}
        <div className="recipe-number-grid">
          <label className="field">
            <span>Startup timeout · seconds</span>
            <input
              type="number"
              min="1"
              value={application.launchRecipe.startupTimeoutSeconds}
              onChange={(event) =>
                update((next) => {
                  next.launchRecipe.startupTimeoutSeconds =
                    event.currentTarget.valueAsNumber || 30;
                })
              }
            />
          </label>
          <label className="field">
            <span>Post-start delay · ms</span>
            <input
              type="number"
              min="0"
              value={application.launchRecipe.postStartDelayMilliseconds}
              onChange={(event) =>
                update((next) => {
                  next.launchRecipe.postStartDelayMilliseconds =
                    event.currentTarget.valueAsNumber || 0;
                })
              }
            />
          </label>
          <label className="field">
            <span>Console</span>
            <select
              value={application.launchRecipe.consoleVisibility}
              onChange={(event) =>
                update((next) => {
                  next.launchRecipe.consoleVisibility = event.currentTarget
                    .value as ProfileApplication["launchRecipe"]["consoleVisibility"];
                })
              }
            >
              <option value="hidden">Hidden</option>
              <option value="visible">Visible</option>
            </select>
          </label>
        </div>
        <div className="supporting-policy">
          <label className="field compact-field">
            <span>Shutdown strategy</span>
            <select
              value={shutdown.kind}
              onChange={(event) =>
                update((next) => {
                  switch (event.currentTarget.value) {
                    case "consoleInterrupt":
                      next.launchRecipe.shutdownStrategy = {
                        kind: "consoleInterrupt",
                      };
                      break;
                    case "customStop":
                      next.launchRecipe.shutdownStrategy = {
                        kind: "customStop",
                        executablePath: "",
                        arguments: [],
                      };
                      break;
                    case "forceOnly":
                      next.launchRecipe.shutdownStrategy = {
                        kind: "forceOnly",
                      };
                      break;
                    default:
                      next.launchRecipe.shutdownStrategy = {
                        kind: "closeWindows",
                      };
                  }
                })
              }
            >
              <option value="closeWindows">Close windows</option>
              <option value="consoleInterrupt">Console interrupt</option>
              <option value="customStop">Custom stop executable</option>
              <option value="forceOnly">No graceful strategy</option>
            </select>
          </label>
          <label className="check-row compact-check">
            <input
              type="checkbox"
              checked={application.launchRecipe.elevated}
              onChange={(event) =>
                update((next) => {
                  next.launchRecipe.elevated = event.currentTarget.checked;
                })
              }
            />
            <span>
              <strong>Launch elevated</strong>
              <small>Uses the one-shot helper in a later milestone.</small>
            </span>
          </label>
        </div>
        {shutdown.kind === "customStop" && (
          <div className="field-grid">
            <label className="field">
              <span>Stop executable path</span>
              <div className="path-input">
                <input
                  value={displayWindowsPath(shutdown.executablePath)}
                  onChange={(event) =>
                    update((next) => {
                      const nextShutdown = next.launchRecipe.shutdownStrategy;
                      if (nextShutdown.kind === "customStop") {
                        nextShutdown.executablePath = event.currentTarget.value;
                      }
                    })
                  }
                />
                <button
                  type="button"
                  className="secondary-button path-browse-button"
                  aria-label={`Browse for ${label} stop executable`}
                  onClick={() =>
                    void selectExecutable(
                      shutdown.kind === "customStop"
                        ? shutdown.executablePath
                        : null,
                      (next, path) => {
                        const nextShutdown = next.launchRecipe.shutdownStrategy;
                        if (nextShutdown.kind === "customStop") {
                          nextShutdown.executablePath = path;
                        }
                      },
                    )
                  }
                >
                  Browseâ€¦
                </button>
              </div>
            </label>
            <label className="field">
              <span>Stop arguments · one per line</span>
              <textarea
                rows={2}
                value={shutdown.arguments.join("\n")}
                onChange={(event) =>
                  update((next) => {
                    const nextShutdown = next.launchRecipe.shutdownStrategy;
                    if (nextShutdown.kind === "customStop") {
                      nextShutdown.arguments = event.currentTarget.value
                        .split("\n")
                        .filter((argument) => argument.length > 0);
                    }
                  })
                }
              />
            </label>
          </div>
        )}
      </div>
    </details>
  );
}

interface DashboardProps {
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

function Dashboard({
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
