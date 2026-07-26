import { useRef, useState, type FormEvent } from "react";
import type {
  AppSnapshot,
  DiscoveredPrimarySim,
  RacingProfile,
} from "../../generated/bindings";
import type { NativeBridge } from "../../native-bridge/native-bridge";
import { discoveredSupportingApplicationToProfile } from "../../ui/presentation";
import type {
  DiscoveryState,
  PrimarySimSource,
  ProfileApproval,
  RecommendationState,
} from "./profile-types";

type ProfileView = "dashboard" | "new-profile" | "edit-profile";

interface ProfileWorkspaceOptions {
  bridge: NativeBridge;
  selectedProfile: RacingProfile | null;
  selectedProfileNeedsReview: boolean;
  onSnapshotChanged(snapshot: AppSnapshot): void;
  onSnapshotError(): void;
  onNavigate(view: ProfileView): void;
  onProfileSelected(): void;
}

export function useProfileWorkspace({
  bridge,
  selectedProfile,
  selectedProfileNeedsReview,
  onSnapshotChanged,
  onSnapshotError,
  onNavigate,
  onProfileSelected,
}: ProfileWorkspaceOptions) {
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
  const [profileIsSaving, setProfileIsSaving] = useState(false);
  const [profileError, setProfileError] = useState<string | null>(null);
  const recommendationRequest = useRef(0);

  const openNewProfile = () => {
    setProfileError(null);
    setProfileName("");
    setPrimarySimName("");
    setPrimarySimSource("direct");
    setSourceValue("");
    setSelectedPrimarySimId(null);
    setSelectedSupportingIds([]);
    setRecommendationState({ kind: "idle" });
    setIsManualEntry(false);
    setDiscoveryState({ kind: "loading" });
    onNavigate("new-profile");
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

  const cancelProfileWizard = () => {
    recommendationRequest.current += 1;
    setDiscoveryState({ kind: "idle" });
    setRecommendationState({ kind: "idle" });
    onNavigate("dashboard");
  };

  const selectProfile = async (profileId: string) => {
    try {
      const nextSnapshot = await bridge.selectProfile({
        profileId,
      });
      onSnapshotChanged(nextSnapshot);
      onNavigate("dashboard");
      setProfileDraft(null);
      onProfileSelected();
    } catch {
      onSnapshotError();
    }
  };

  const createProfile = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setProfileIsSaving(true);
    setProfileError(null);

    try {
      const nextSnapshot = await bridge.createProfile({
        profile: {
          name: profileName,
          primarySim: {
            name: primarySimName,
            launchRecipe: {
              source:
                primarySimSource === "steam"
                  ? {
                      kind: "steam",
                      appId: Number.parseInt(sourceValue, 10) || 0,
                      selector: null,
                    }
                  : {
                      kind: "directExecutable",
                      executablePath: sourceValue,
                    },
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
          },
          supportingApplications:
            discoveryState.kind === "ready"
              ? discoveryState.snapshot.installedSupportingApplications
                  .filter((application) =>
                    selectedSupportingIds.includes(application.id),
                  )
                  .map(discoveredSupportingApplicationToProfile)
              : [],
          vrEnabled: false,
          preferredVrLaunchMode: null,
          closeSession: { stopSteamVr: false },
        },
      });
      onSnapshotChanged(nextSnapshot);
      onNavigate("dashboard");
      setProfileName("");
      setPrimarySimName("");
      setSourceValue("");
      setDiscoveryState({ kind: "idle" });
      setRecommendationState({ kind: "idle" });
      setSelectedPrimarySimId(null);
      setSelectedSupportingIds([]);
      setIsManualEntry(false);
    } catch {
      setProfileError(
        "The Racing Profile could not be created. Review the profile details and try again.",
      );
    } finally {
      setProfileIsSaving(false);
    }
  };

  const openProfileEditor = () => {
    if (!selectedProfile) {
      return;
    }
    setProfileError(null);
    setProfileDraft(structuredClone(selectedProfile));
    onNavigate("edit-profile");
  };

  const pickExecutablePath = async (
    initialPath?: string | null,
  ): Promise<string | null> => {
    try {
      return await bridge.pickExecutablePath(initialPath);
    } catch {
      setProfileError(
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
    setProfileIsSaving(true);
    setProfileError(null);
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
      onSnapshotChanged(nextSnapshot);
      setProfileDraft(null);
      onNavigate("dashboard");
    } catch {
      setProfileError(
        selectedProfileNeedsReview
          ? "The Racing Profile is still quarantined. Repair missing paths and approve every elevated or custom-stop entry."
          : "The Racing Profile could not be saved. Review the profile details and try again.",
      );
    } finally {
      setProfileIsSaving(false);
    }
  };

  const openDuplicateProfile = () => {
    if (!selectedProfile) {
      return;
    }
    setProfileError(null);
    setDuplicateName(`${selectedProfile.name} Copy`);
    setIsDuplicateOpen(true);
  };

  const duplicateProfile = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selectedProfile) {
      return;
    }
    setProfileIsSaving(true);
    setProfileError(null);
    try {
      const nextSnapshot = await bridge.duplicateProfile({
        sourceProfileId: selectedProfile.id,
        name: duplicateName,
      });
      onSnapshotChanged(nextSnapshot);
      setIsDuplicateOpen(false);
      setDuplicateName("");
    } catch {
      setProfileError(
        "The Racing Profile could not be duplicated. Choose a different name and try again.",
      );
    } finally {
      setProfileIsSaving(false);
    }
  };

  const deleteProfile = async () => {
    if (!selectedProfile) {
      return;
    }
    setProfileIsSaving(true);
    setProfileError(null);
    try {
      const nextSnapshot = await bridge.deleteProfile({
        profileId: selectedProfile.id,
      });
      onSnapshotChanged(nextSnapshot);
      setIsDeleteOpen(false);
    } catch {
      setProfileError(
        "The Racing Profile could not be deleted. Close this dialog and try again.",
      );
    } finally {
      setProfileIsSaving(false);
    }
  };

  const exportProfile = async () => {
    if (!selectedProfile) {
      return;
    }
    setProfileIsSaving(true);
    setProfileError(null);
    setExportDocument("");
    setIsExportOpen(true);
    try {
      const document = await bridge.exportProfile({
        profileId: selectedProfile.id,
      });
      setExportDocument(document);
    } catch {
      setProfileError(
        "The Racing Profile could not be exported. Close this dialog and try again.",
      );
    } finally {
      setProfileIsSaving(false);
    }
  };

  const importProfile = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setProfileIsSaving(true);
    setProfileError(null);
    try {
      const nextSnapshot = await bridge.importProfile({
        document: importDocument,
      });
      onSnapshotChanged(nextSnapshot);
      setIsImportOpen(false);
      setImportDocument("");
    } catch {
      setProfileError(
        "The Racing Profile could not be imported. Check the portable JSON and try again.",
      );
    } finally {
      setProfileIsSaving(false);
    }
  };

  return {
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
  };
}
