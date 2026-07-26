import { useState } from "react";
import type {
  ApplicationProcessSnapshot,
  AppSnapshot,
  GameLaunchDiagnostic,
  ProfileApplication,
  RacingProfile,
} from "../../generated/bindings";
import type { NativeBridge } from "../../native-bridge/native-bridge";
import { commandErrorMessage } from "../../ui/presentation";

type PendingProcessAction = {
  kind: "exit" | "restart" | "force";
  application: ProfileApplication;
  process: ApplicationProcessSnapshot;
};

interface DashboardControllerOptions {
  bridge: NativeBridge;
  selectedProfile: RacingProfile | null;
  snapshot: AppSnapshot | null;
  onSnapshotChanged(snapshot: AppSnapshot): void;
}

export function useDashboardController({
  bridge,
  selectedProfile,
  snapshot,
  onSnapshotChanged,
}: DashboardControllerOptions) {
  const [dashboardIsBusy, setDashboardIsBusy] = useState(false);
  const [dashboardError, setDashboardError] = useState<string | null>(null);
  const [pendingProcessAction, setPendingProcessAction] =
    useState<PendingProcessAction | null>(null);
  const [outputApplication, setOutputApplication] =
    useState<ProfileApplication | null>(null);
  const [gameLaunchDiagnostic, setGameLaunchDiagnostic] =
    useState<GameLaunchDiagnostic | null>(null);

  const startApplication = async (application: ProfileApplication) => {
    if (!selectedProfile) {
      return;
    }
    setDashboardIsBusy(true);
    setDashboardError(null);
    try {
      const nextSnapshot = await bridge.startApplication({
        profileId: selectedProfile.id,
        applicationId: application.id,
      });
      onSnapshotChanged(nextSnapshot);
    } catch (error) {
      setDashboardError(
        commandErrorMessage(
          error,
          `${application.name} could not start. Check its Launch Recipe and try again.`,
        ),
      );
    } finally {
      setDashboardIsBusy(false);
    }
  };

  const testGameLaunch = async () => {
    if (!selectedProfile) {
      return;
    }
    setDashboardIsBusy(true);
    setDashboardError(null);
    setGameLaunchDiagnostic(null);
    try {
      const diagnostic = await bridge.testGameLaunch({
        profileId: selectedProfile.id,
      });
      setGameLaunchDiagnostic(diagnostic);
    } catch {
      setDashboardError(
        "Test Game Launch could not start the Primary Sim. Review its Launch Recipe and try again.",
      );
    } finally {
      setDashboardIsBusy(false);
    }
  };

  const toggleDashboardVr = async (vrEnabled: boolean) => {
    if (!selectedProfile || snapshot?.session.state !== "idle") {
      return;
    }
    setDashboardIsBusy(true);
    setDashboardError(null);
    try {
      const profile = structuredClone(selectedProfile);
      profile.vrEnabled = vrEnabled;
      const nextSnapshot = await bridge.saveProfile({ profile });
      onSnapshotChanged(nextSnapshot);
      setGameLaunchDiagnostic(null);
    } catch {
      setDashboardError("Formation Lap could not remember the VR choice.");
    } finally {
      setDashboardIsBusy(false);
    }
  };

  const runSessionAction = async (
    action: "start" | "cancel" | "close" | "acceptRecovery" | "dismissRecovery",
  ) => {
    if (action === "start" && !selectedProfile) {
      return;
    }
    setDashboardIsBusy(true);
    setDashboardError(null);
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
      onSnapshotChanged(nextSnapshot);
    } catch (error) {
      setDashboardError(
        commandErrorMessage(
          error,
          "Formation Lap could not complete the Session action.",
        ),
      );
    } finally {
      setDashboardIsBusy(false);
    }
  };

  const exitApplication = async (
    application: ProfileApplication,
    preExistingConfirmed: boolean,
  ) => {
    setDashboardIsBusy(true);
    setDashboardError(null);
    try {
      const nextSnapshot = await bridge.exitApplication({
        applicationId: application.id,
        preExistingConfirmed,
      });
      onSnapshotChanged(nextSnapshot);
      const nextProcess = nextSnapshot.applicationProcesses.find(
        (process) => process.applicationId === application.id,
      );
      if (nextProcess?.status === "stopping") {
        setPendingProcessAction({
          kind: "force",
          application,
          process: nextProcess,
        });
      }
    } catch {
      setDashboardError(
        `${application.name} did not stop. Review its shutdown strategy and try again.`,
      );
    } finally {
      setDashboardIsBusy(false);
    }
  };

  const restartApplication = async (
    application: ProfileApplication,
    preExistingConfirmed: boolean,
  ) => {
    if (!selectedProfile) {
      return;
    }
    setDashboardIsBusy(true);
    setDashboardError(null);
    try {
      const nextSnapshot = await bridge.restartApplication({
        profileId: selectedProfile.id,
        applicationId: application.id,
        preExistingConfirmed,
      });
      onSnapshotChanged(nextSnapshot);
      const nextProcess = nextSnapshot.applicationProcesses.find(
        (process) => process.applicationId === application.id,
      );
      if (nextProcess?.status === "stopping") {
        setPendingProcessAction({
          kind: "force",
          application,
          process: nextProcess,
        });
      }
    } catch {
      setDashboardError(
        `${application.name} could not restart. Check its Launch Recipe and try again.`,
      );
    } finally {
      setDashboardIsBusy(false);
    }
  };

  const forceStopApplication = async (
    application: ProfileApplication,
    processSnapshot: ApplicationProcessSnapshot,
  ) => {
    setDashboardIsBusy(true);
    setDashboardError(null);
    try {
      const nextSnapshot = await bridge.forceStopApplication({
        applicationId: application.id,
        preExistingConfirmed: processSnapshot.ownership === "preExisting",
        forceConfirmed: true,
      });
      onSnapshotChanged(nextSnapshot);
      setPendingProcessAction(null);
    } catch {
      setDashboardError(
        `${application.name} could not be force stopped. Try again or close it directly.`,
      );
    } finally {
      setDashboardIsBusy(false);
    }
  };

  const requestProcessAction = (
    kind: PendingProcessAction["kind"],
    application: ProfileApplication,
    processSnapshot: ApplicationProcessSnapshot,
  ) => {
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

  const installFormationLapUpdate = async () => {
    setDashboardIsBusy(true);
    setDashboardError(null);
    try {
      const nextSnapshot = await bridge.installFormationLapUpdate();
      onSnapshotChanged(nextSnapshot);
    } catch {
      setDashboardError(
        "Formation Lap rejected the update or the Session is not idle.",
      );
    } finally {
      setDashboardIsBusy(false);
    }
  };

  const clearGameLaunchDiagnostic = () => setGameLaunchDiagnostic(null);

  return {
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
    installFormationLapUpdate,
  };
}
